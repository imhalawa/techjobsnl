using FluentAssertions;
using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Providers;

namespace TechJobsNL.Core.Tests.Providers;

public sealed class VacancyProviderContractTests
{
    [Fact]
    [Trait("TaskId", "V0.1.0-008")]
    public void From_EmptyCompleteScan_IsAValidExplicitOutcome()
    {
        ProviderScanResult.From(new SourceScan.Complete([])).Should().BeOfType<ProviderScanResult.Accepted>();
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-008")]
    public void From_IncompleteScan_PreservesCompletenessAndObservations()
    {
        var scan = new SourceScan.Incomplete([Observation()], "page limit reached");

        ProviderScanResult.From(scan).Should().Be(new ProviderScanResult.Accepted(scan));
    }

    [Theory]
    [Trait("TaskId", "V0.1.0-008")]
    [InlineData("", "https://example.test/jobs/1", "https://example.test/jobs/1/apply", "{}")]
    [InlineData("Engineer", "transport://response", "https://example.test/jobs/1/apply", "{}")]
    [InlineData("Engineer", "https://example.test/jobs/1", "https://example.test/jobs/1/apply", "")]
    public void From_MalformedObservation_ReturnsSchemaFailure(string title, string jobUrl, string applyUrl, string rawPayload)
    {
        var outcome = ProviderScanResult.From(new SourceScan.Complete([Observation(title, jobUrl, applyUrl, rawPayload)]));

        outcome.Should().BeOfType<ProviderScanResult.Failed>().Which.Failure.Kind.Should().Be(SourceErrorKind.Schema);
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-008")]
    public async Task ScanAsync_TestAdapter_ObservesCancellationAndUsesTheCoreSeam()
    {
        using var cancellation = new CancellationTokenSource();
        await cancellation.CancelAsync();
        IVacancyProvider provider = new DeterministicProvider();

        var action = () => provider.ScanAsync(cancellation.Token);

        await action.Should().ThrowAsync<OperationCanceledException>();
        provider.CompanyId.Should().Be(new CompanyId("example"));
    }

    private static ObservedVacancy Observation(string title = "Engineer", string jobUrl = "https://example.test/jobs/1", string applyUrl = "https://example.test/jobs/1/apply", string rawPayload = "{}") =>
        new(new SourceId("one"), title, null, null, null, ["Amsterdam"], ["NL"], jobUrl, applyUrl, "Description", rawPayload, null);

    private sealed class DeterministicProvider : IVacancyProvider
    {
        public CompanyId CompanyId => new("example");

        public Task<ProviderScanResult> ScanAsync(CancellationToken cancellationToken)
        {
            cancellationToken.ThrowIfCancellationRequested();
            return Task.FromResult(ProviderScanResult.From(new SourceScan.Complete([Observation()])));
        }
    }
}
