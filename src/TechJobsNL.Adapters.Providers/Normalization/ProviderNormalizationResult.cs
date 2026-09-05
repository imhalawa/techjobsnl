using TechJobsNL.Adapters.Providers.Http;

namespace TechJobsNL.Adapters.Providers.Normalization;

public abstract record ProviderNormalizationResult<T>
{
    private ProviderNormalizationResult() { }

    public sealed record Success(T Value) : ProviderNormalizationResult<T>;

    public sealed record Failure(ProviderFailure Error) : ProviderNormalizationResult<T>;
}
