using System.Collections.Concurrent;
using TechJobsNL.Core.Application.Progress;
using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Domain.Configuration;
using TechJobsNL.Core.Eligibility;
using TechJobsNL.Core.Providers;

namespace TechJobsNL.Core.Scanning;

/// <summary>Runs explicit company scans with bounded fetches and company-local persistence.</summary>
public sealed class ScanOrchestrator
{
    private readonly EligibilityClassifier classifier;
    private readonly IScanPersistence persistence;
    private readonly TimeProvider timeProvider;
    private readonly Func<TimeSpan, CancellationToken, Task> delay;

    public ScanOrchestrator(EligibilityClassifier classifier, IScanPersistence persistence, TimeProvider timeProvider, Func<TimeSpan, CancellationToken, Task>? delay = null)
    {
        this.classifier = classifier;
        this.persistence = persistence;
        this.timeProvider = timeProvider;
        this.delay = delay ?? Task.Delay;
    }

    public async Task<ScanRunSummary> RunAsync(string runId, IReadOnlyCollection<CompanyConfiguration> companies, IReadOnlyCollection<IVacancyProvider> providers, ScanConfiguration settings, IProgressStream<ScanEvent> progress, CancellationToken cancellationToken)
    {
        var configurations = companies.ToDictionary(static company => new CompanyId(company.Id));
        var scheduled = providers.Where(provider => configurations.TryGetValue(provider.CompanyId, out var company) && company.Enabled).ToArray();
        var publishLock = new SemaphoreSlim(1, 1);
        var outcomes = new ConcurrentBag<Outcome>();
        await PublishAsync(progress, new RunStarted(runId, scheduled.Length), publishLock, cancellationToken).ConfigureAwait(false);
        try
        {
            await Parallel.ForEachAsync(scheduled, new ParallelOptions { MaxDegreeOfParallelism = settings.Concurrency, CancellationToken = cancellationToken }, async (provider, token) =>
            {
                await PublishAsync(progress, new CompanyStarted(provider.CompanyId), publishLock, token).ConfigureAwait(false);
                var startedAt = timeProvider.GetUtcNow();
                var company = configurations[provider.CompanyId];
                var outcome = await ScanOneAsync(provider, company, settings, token).ConfigureAwait(false);
                var completedAt = timeProvider.GetUtcNow();
                var terminal = await PersistAsync(runId, company, provider.CompanyId, outcome, startedAt, completedAt, token).ConfigureAwait(false);
                outcomes.Add(terminal);
                await PublishAsync(progress, terminal.Event, publishLock, token).ConfigureAwait(false);
            }).ConfigureAwait(false);
            var summary = new ScanRunSummary(outcomes.Count(static outcome => outcome.Kind == OutcomeKind.Complete), outcomes.Count(static outcome => outcome.Kind == OutcomeKind.Failed), outcomes.Count(static outcome => outcome.Kind == OutcomeKind.Incomplete));
            await PublishAsync(progress, new RunFinished(runId, summary.Completed, summary.Failed, summary.Incomplete), publishLock, cancellationToken).ConfigureAwait(false);
            return summary;
        }
        finally
        {
            progress.Complete();
            publishLock.Dispose();
        }
    }

    private async Task<Outcome> ScanOneAsync(IVacancyProvider provider, CompanyConfiguration company, ScanConfiguration settings, CancellationToken cancellationToken)
    {
        for (var retriesUsed = 0; ; retriesUsed++)
        {
            ProviderScanResult result;
            using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            timeout.CancelAfter(TimeSpan.FromSeconds(settings.TimeoutSeconds));
            try { result = await provider.ScanAsync(timeout.Token).ConfigureAwait(false); }
            catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested) { result = ProviderScanResult.Fail(new ScanFailure(SourceErrorKind.Timeout, $"scan attempt timed out after {settings.TimeoutSeconds} seconds", null, null, true)); }
            catch (Exception exception) { result = ProviderScanResult.Fail(new ScanFailure(SourceErrorKind.Transport, exception.Message)); }
            if (result is ProviderScanResult.Failed failed && retriesUsed < settings.RetryCount && ShouldRetry(failed.Failure))
            {
                await delay(failed.Failure.RetryAfter ?? RetryDelay(provider.CompanyId, retriesUsed), cancellationToken).ConfigureAwait(false);
                continue;
            }
            return Classify(provider.CompanyId, company, result);
        }
    }

    private Outcome Classify(CompanyId companyId, CompanyConfiguration company, ProviderScanResult result)
    {
        if (result is ProviderScanResult.Failed failed) return Outcome.Failed(companyId, failed.Failure);
        var scan = ((ProviderScanResult.Accepted)result).Scan;
        var observations = scan is SourceScan.Complete complete ? complete.Observations : ((SourceScan.Incomplete)scan).Observations;
        var diagnostics = new List<string>();
        if (scan is SourceScan.Incomplete incomplete) diagnostics.Add(incomplete.Diagnostic);
        var classified = new List<ClassifiedVacancy>(observations.Length);
        foreach (var observation in observations)
        {
            var classification = classifier.Classify(observation, company.LocationCountryOverrides);
            if (classification is EligibilityClassification.Incomplete unresolved) diagnostics.Add($"unresolved location labels: [{string.Join(", ", unresolved.UnresolvedLocations)}]");
            else classified.Add(new ClassifiedVacancy(observation, ((EligibilityClassification.Decided)classification).Eligibility));
        }
        return diagnostics.Count > 0 ? Outcome.Incomplete(companyId, observations.Length, string.Join("; ", diagnostics)) : Outcome.Complete(companyId, classified);
    }

    private async Task<Outcome> PersistAsync(string runId, CompanyConfiguration company, CompanyId companyId, Outcome outcome, DateTimeOffset startedAt, DateTimeOffset completedAt, CancellationToken cancellationToken)
    {
        try
        {
            if (outcome.Kind == OutcomeKind.Complete) await persistence.PersistCompleteAsync(runId, company, outcome.Vacancies, startedAt, completedAt, cancellationToken).ConfigureAwait(false);
            else if (outcome.Kind == OutcomeKind.Incomplete) await persistence.PersistIncompleteAsync(runId, companyId, outcome.Diagnostic!, outcome.ObservedCount, startedAt, completedAt, cancellationToken).ConfigureAwait(false);
            else await persistence.PersistFailedAsync(runId, companyId, outcome.Failure!, startedAt, completedAt, cancellationToken).ConfigureAwait(false);
            return outcome;
        }
        catch (Exception exception) when (exception is not OperationCanceledException)
        {
            return Outcome.Failed(companyId, new ScanFailure(SourceErrorKind.Storage, $"could not record scan for {companyId.Value}: {exception.Message}"));
        }
    }

    private static bool ShouldRetry(ScanFailure failure) => failure.IsRetryable && (failure.HttpStatus == 429 || failure.Kind == SourceErrorKind.Timeout || failure.Kind == SourceErrorKind.Transport && failure.HttpStatus is >= 500 and <= 599);
    private static TimeSpan RetryDelay(CompanyId companyId, int retriesUsed) { var seed = companyId.Value.Aggregate((long)retriesUsed + 1, static (value, character) => unchecked(value * 31 + character)); return TimeSpan.FromMilliseconds((retriesUsed == 0 ? 250 : 500) + (seed & long.MaxValue) % 26); }
    private static async Task PublishAsync(IProgressStream<ScanEvent> progress, ScanEvent value, SemaphoreSlim publishLock, CancellationToken cancellationToken) { await publishLock.WaitAsync(cancellationToken).ConfigureAwait(false); try { await progress.PublishAsync(value, cancellationToken).ConfigureAwait(false); } finally { publishLock.Release(); } }

    private enum OutcomeKind { Complete, Failed, Incomplete }
    private sealed record Outcome(OutcomeKind Kind, ScanEvent Event, IReadOnlyCollection<ClassifiedVacancy> Vacancies, ScanFailure? Failure, string? Diagnostic, int ObservedCount)
    {
        public static Outcome Complete(CompanyId id, IReadOnlyCollection<ClassifiedVacancy> vacancies) => new(OutcomeKind.Complete, new CompanyCompleted(id, vacancies.Count, vacancies.Count(static vacancy => vacancy.Eligibility.IsEligible)), vacancies, null, null, vacancies.Count);
        public static Outcome Failed(CompanyId id, ScanFailure failure) => new(OutcomeKind.Failed, new CompanyFailed(id, failure.Kind, failure.Diagnostic), [], failure, null, 0);
        public static Outcome Incomplete(CompanyId id, int count, string diagnostic) => new(OutcomeKind.Incomplete, new CompanyIncomplete(id, diagnostic, count), [], null, diagnostic, count);
    }
}
