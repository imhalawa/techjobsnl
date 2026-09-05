using System.Collections.Concurrent;
using System.Collections.Immutable;
using FluentAssertions;
using TechJobsNL.Core.Application.Progress;
using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Domain.Configuration;
using TechJobsNL.Core.Eligibility;
using TechJobsNL.Core.Providers;
using TechJobsNL.Core.Scanning;

namespace TechJobsNL.Core.Tests.Scanning;

public sealed class ScanOrchestratorTests
{
    [Fact]
    [Trait("TaskId", "V0.1.0-017")]
    public async Task RunAsync_BoundsFetchesAndOrdersProgressLifecycle()
    {
        var concurrency = new ConcurrencyProbe();
        var providers = new[] { "one", "two", "three", "four" }.Select(id => new FakeProvider(id, async token => { await concurrency.EnterAsync(token).ConfigureAwait(false); return Complete(id); })).Cast<IVacancyProvider>().ToArray();
        var progress = ProgressStreams.Create<ScanEvent>(20);
        var persistence = new FakePersistence();

        var summary = await Subject(persistence).RunAsync("run-1", providers.Select(provider => Company(provider.CompanyId.Value)).ToArray(), providers, Settings(concurrency: 2), progress, TestContext.Current.CancellationToken);
        var events = await DrainAsync(progress);

        summary.Should().Be(new ScanRunSummary(4, 0, 0));
        concurrency.Maximum.Should().Be(2);
        events.First().Should().Be(new RunStarted("run-1", 4));
        events.Last().Should().Be(new RunFinished("run-1", 4, 0, 0));
        foreach (var provider in providers) events.IndexOf(events.OfType<CompanyStarted>().Single(item => item.CompanyId == provider.CompanyId)).Should().BeLessThan(events.IndexOf(events.OfType<CompanyCompleted>().Single(item => item.CompanyId == provider.CompanyId)));
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-017")]
    public async Task RunAsync_RetriesExactEligibleFailuresUsingRetryAfterOrFallback()
    {
        var delays = new ConcurrentQueue<TimeSpan>();
        var timeout = new SequenceProvider("timeout", ProviderScanResult.Fail(new ScanFailure(SourceErrorKind.Timeout, "slow", null, TimeSpan.FromMilliseconds(600), true)), Complete("one"));
        var server = new SequenceProvider("server", ProviderScanResult.Fail(new ScanFailure(SourceErrorKind.Transport, "busy", 503, null, true)), Complete("two"));
        var client = new SequenceProvider("client", ProviderScanResult.Fail(new ScanFailure(SourceErrorKind.Transport, "missing", 404, null, true)), Complete("three"));
        IVacancyProvider[] providers = [timeout, server, client];
        var progress = ProgressStreams.Create<ScanEvent>(20);

        var summary = await Subject(new FakePersistence(), (delay, _) => { delays.Enqueue(delay); return Task.CompletedTask; }).RunAsync("run", providers.Select(provider => Company(provider.CompanyId.Value)).ToArray(), providers, Settings(retries: 1), progress, TestContext.Current.CancellationToken);

        summary.Should().Be(new ScanRunSummary(2, 1, 0));
        timeout.Attempts.Should().Be(2); server.Attempts.Should().Be(2); client.Attempts.Should().Be(1);
        delays.Should().Contain(TimeSpan.FromMilliseconds(600));
        delays.Should().Contain(delay => delay >= TimeSpan.FromMilliseconds(250) && delay <= TimeSpan.FromMilliseconds(275));
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-017")]
    public async Task RunAsync_SourceAndClassificationIncompleteNeverPersistVacancies()
    {
        var sourcePartial = new FakeProvider("partial", _ => Task.FromResult<ProviderScanResult>(new ProviderScanResult.Accepted(new SourceScan.Incomplete([Observation("one", ["NL"])], "truncated"))));
        var unresolved = new FakeProvider("unresolved", _ => Task.FromResult(Complete("two", [])));
        IVacancyProvider[] providers = [sourcePartial, unresolved];
        var persistence = new FakePersistence();
        var progress = ProgressStreams.Create<ScanEvent>(20);

        var summary = await Subject(persistence).RunAsync("run", providers.Select(provider => Company(provider.CompanyId.Value)).ToArray(), providers, Settings(), progress, TestContext.Current.CancellationToken);

        summary.Should().Be(new ScanRunSummary(0, 0, 2));
        persistence.CompleteCompanies.Should().BeEmpty();
        persistence.IncompleteCompanies.Should().BeEquivalentTo([new CompanyId("partial"), new CompanyId("unresolved")]);
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-017")]
    public async Task RunAsync_StorageFailureIsIsolatedAndSummaryStaysHonest()
    {
        IVacancyProvider[] providers = [new FakeProvider("healthy", _ => Task.FromResult(Complete("one"))), new FakeProvider("broken", _ => Task.FromResult(Complete("two")))];
        var persistence = new FakePersistence(new CompanyId("broken"));
        var progress = ProgressStreams.Create<ScanEvent>(20);

        var summary = await Subject(persistence).RunAsync("run", providers.Select(provider => Company(provider.CompanyId.Value)).ToArray(), providers, Settings(), progress, TestContext.Current.CancellationToken);
        var events = await DrainAsync(progress);

        summary.Should().Be(new ScanRunSummary(1, 1, 0));
        events.OfType<CompanyFailed>().Single().Kind.Should().Be(SourceErrorKind.Storage);
        persistence.CompleteCompanies.Should().Contain(new CompanyId("healthy"));
    }

    private static ScanOrchestrator Subject(IScanPersistence persistence, Func<TimeSpan, CancellationToken, Task>? delay = null) => new(Classifier(), persistence, TimeProvider.System, delay);
    private static EligibilityClassifier Classifier() => ((EligibilityClassifierCreation.Ready)EligibilityClassifier.Create(new FiltersConfiguration(["NL"], 7, [], []))).Classifier;
    private static ScanConfiguration Settings(int concurrency = 2, int retries = 0) => new(concurrency, 20, retries, "test");
    private static CompanyConfiguration Company(string id) => new(id, id, "Test", "Test", true, ImmutableDictionary<string, string>.Empty, new SourceConfiguration.Unsupported("fixture"));
    private static ProviderScanResult Complete(string id, ImmutableArray<string>? countries = null) => new ProviderScanResult.Accepted(new SourceScan.Complete([Observation(id, countries ?? ["NL"])]));
    private static ObservedVacancy Observation(string id, ImmutableArray<string> countries) => new(new SourceId(id), "Engineer", null, null, null, ["Amsterdam"], countries, $"https://example.test/{id}", $"https://example.test/{id}/apply", "Description", "{}", null);
    private static async Task<List<ScanEvent>> DrainAsync(IProgressStream<ScanEvent> progress) { var values = new List<ScanEvent>(); await foreach (var value in progress.Reader.ReadAllAsync(TestContext.Current.CancellationToken).ConfigureAwait(false)) values.Add(value); return values; }

    private sealed class FakeProvider(string id, Func<CancellationToken, Task<ProviderScanResult>> scan) : IVacancyProvider { public CompanyId CompanyId { get; } = new(id); public Task<ProviderScanResult> ScanAsync(CancellationToken cancellationToken) => scan(cancellationToken); }
    private sealed class SequenceProvider(string id, params ProviderScanResult[] results) : IVacancyProvider { private int index; public int Attempts { get; private set; } public CompanyId CompanyId { get; } = new(id); public Task<ProviderScanResult> ScanAsync(CancellationToken cancellationToken) { cancellationToken.ThrowIfCancellationRequested(); Attempts++; return Task.FromResult(results[Math.Min(index++, results.Length - 1)]); } }
    private sealed class FakePersistence(CompanyId? fail = null) : IScanPersistence
    {
        public ConcurrentBag<CompanyId> CompleteCompanies { get; } = [];
        public ConcurrentBag<CompanyId> IncompleteCompanies { get; } = [];
        public Task PersistCompleteAsync(string runId, CompanyConfiguration company, IReadOnlyCollection<ClassifiedVacancy> vacancies, DateTimeOffset startedAt, DateTimeOffset completedAt, CancellationToken cancellationToken) { cancellationToken.ThrowIfCancellationRequested(); var id = new CompanyId(company.Id); if (id == fail) throw new IOException("injected"); CompleteCompanies.Add(id); return Task.CompletedTask; }
        public Task PersistIncompleteAsync(string runId, CompanyId companyId, string diagnostic, int observedCount, DateTimeOffset startedAt, DateTimeOffset completedAt, CancellationToken cancellationToken) { cancellationToken.ThrowIfCancellationRequested(); IncompleteCompanies.Add(companyId); return Task.CompletedTask; }
        public Task PersistFailedAsync(string runId, CompanyId companyId, ScanFailure failure, DateTimeOffset startedAt, DateTimeOffset completedAt, CancellationToken cancellationToken) => Task.CompletedTask;
    }
    private sealed class ConcurrencyProbe { private int current; public int Maximum { get; private set; } public async Task EnterAsync(CancellationToken token) { var value = Interlocked.Increment(ref current); Maximum = Math.Max(Maximum, value); await Task.Delay(30, token).ConfigureAwait(false); Interlocked.Decrement(ref current); } }
}
