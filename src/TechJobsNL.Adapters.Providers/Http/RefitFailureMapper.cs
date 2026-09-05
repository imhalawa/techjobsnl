using System.Net;
using Refit;
using TechJobsNL.Core.Domain;

namespace TechJobsNL.Adapters.Providers.Http;

/// <summary>Maps Refit failures into the shared provider failure vocabulary.</summary>
public static class RefitFailureMapper
{
    public static ProviderFailure Map(ApiException exception, string source)
    {
        var status = (int)exception.StatusCode;
        var kind = exception.StatusCode switch { HttpStatusCode.TooManyRequests => SourceErrorKind.RateLimit, HttpStatusCode.RequestTimeout => SourceErrorKind.Timeout, _ => SourceErrorKind.Transport };
        var retryable = exception.StatusCode is HttpStatusCode.TooManyRequests or HttpStatusCode.RequestTimeout || status >= 500;
        var message = exception.StatusCode == HttpStatusCode.TooManyRequests ? $"{source} rate limit exceeded" : $"{source} returned HTTP {status}";
        return new ProviderFailure(kind, message, status, exception.Headers?.RetryAfter?.Delta, retryable);
    }
}
