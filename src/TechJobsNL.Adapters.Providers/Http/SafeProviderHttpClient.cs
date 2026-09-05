using System.Net;
using TechJobsNL.Core.Domain;

namespace TechJobsNL.Adapters.Providers.Http;

/// <summary>Sends explicit provider requests while enforcing HTTPS origins and stable failure mapping.</summary>
public sealed class SafeProviderHttpClient
{
    private readonly HttpClient client;
    private readonly HashSet<string> approvedHosts;
    private readonly TimeProvider timeProvider;

    public SafeProviderHttpClient(HttpClient client, IEnumerable<string> approvedHosts, TimeProvider? timeProvider = null)
    {
        this.client = client;
        this.approvedHosts = approvedHosts.ToHashSet(StringComparer.OrdinalIgnoreCase);
        this.timeProvider = timeProvider ?? TimeProvider.System;
    }

    public async Task<ProviderHttpResult> GetTextAsync(Uri uri, string source, CancellationToken cancellationToken)
    {
        if (!IsApproved(uri)) return Unsafe(uri);
        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, uri);
            using var response = await client.SendAsync(request, HttpCompletionOption.ResponseHeadersRead, cancellationToken).ConfigureAwait(false);
            var finalUri = response.RequestMessage?.RequestUri ?? uri;
            if (!IsApproved(finalUri)) return Unsafe(finalUri);
            if (!response.IsSuccessStatusCode) return new ProviderHttpResult.Failure(StatusFailure(response, source));
            return new ProviderHttpResult.Success(await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false), CanonicalUrls.Normalize(finalUri));
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            return Failure(SourceErrorKind.Timeout, $"{source} request timed out", null, null, true);
        }
        catch (HttpRequestException exception)
        {
            return Failure(SourceErrorKind.Transport, exception.Message, (int?)exception.StatusCode, null, true);
        }
    }

    private ProviderFailure StatusFailure(HttpResponseMessage response, string source)
    {
        var status = (int)response.StatusCode;
        var kind = response.StatusCode switch { HttpStatusCode.TooManyRequests => SourceErrorKind.RateLimit, HttpStatusCode.RequestTimeout => SourceErrorKind.Timeout, _ => SourceErrorKind.Transport };
        var retryable = response.StatusCode is HttpStatusCode.TooManyRequests or HttpStatusCode.RequestTimeout || status >= 500;
        var message = response.StatusCode == HttpStatusCode.TooManyRequests ? $"{source} rate limit exceeded" : $"{source} returned HTTP {status}";
        return new ProviderFailure(kind, message, status, ParseRetryAfter(response), retryable);
    }

    private TimeSpan? ParseRetryAfter(HttpResponseMessage response)
    {
        var retry = response.Headers.RetryAfter;
        if (retry?.Delta is { } delta) return delta;
        if (retry?.Date is not { } date) return null;
        var remaining = date - timeProvider.GetUtcNow();
        return remaining > TimeSpan.Zero ? remaining : TimeSpan.Zero;
    }

    private bool IsApproved(Uri uri) => uri.IsAbsoluteUri && string.Equals(uri.Scheme, Uri.UriSchemeHttps, StringComparison.Ordinal) && approvedHosts.Contains(uri.IdnHost);
    private static ProviderHttpResult.Failure Unsafe(Uri uri) => Failure(SourceErrorKind.Configuration, $"unsafe provider URI: {uri.GetLeftPart(UriPartial.Authority)}", null, null, false);
    private static ProviderHttpResult.Failure Failure(SourceErrorKind kind, string message, int? status, TimeSpan? retryAfter, bool retryable) => new(new ProviderFailure(kind, message, status, retryAfter, retryable));
}
