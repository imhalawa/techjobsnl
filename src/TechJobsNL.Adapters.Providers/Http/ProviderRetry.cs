using Polly;
using Polly.Retry;

namespace TechJobsNL.Adapters.Providers.Http;

/// <summary>Defines the shared retry decision consumed by provider resilience pipelines.</summary>
public static class ProviderRetry
{
    public static bool ShouldRetry(ProviderHttpResult result) => result is ProviderHttpResult.Failure { Error.IsRetryable: true };

    public static ResiliencePipeline<ProviderHttpResult> CreatePipeline(int maximumRetryAttempts, TimeSpan delay) =>
        new ResiliencePipelineBuilder<ProviderHttpResult>()
            .AddRetry(new RetryStrategyOptions<ProviderHttpResult>
            {
                MaxRetryAttempts = maximumRetryAttempts,
                Delay = delay,
                ShouldHandle = new PredicateBuilder<ProviderHttpResult>().HandleResult(ShouldRetry)
            })
            .Build();
}
