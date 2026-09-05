namespace TechJobsNL.Adapters.Providers.Http;

/// <summary>Represents a text response or a classified provider failure.</summary>
public abstract record ProviderHttpResult
{
    private ProviderHttpResult() { }

    public sealed record Success(string Body, Uri CanonicalUri) : ProviderHttpResult;

    public sealed record Failure(ProviderFailure Error) : ProviderHttpResult;
}
