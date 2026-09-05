namespace TechJobsNL.Adapters.Providers.Http;

/// <summary>Normalizes trusted URLs for stable identity and comparison.</summary>
public static class CanonicalUrls
{
    public static Uri Normalize(Uri value)
    {
        var builder = new UriBuilder(value) { Fragment = string.Empty, Host = value.IdnHost.ToLowerInvariant() };
        if ((string.Equals(builder.Scheme, Uri.UriSchemeHttps, StringComparison.Ordinal) && builder.Port == 443) ||
            (string.Equals(builder.Scheme, Uri.UriSchemeHttp, StringComparison.Ordinal) && builder.Port == 80)) builder.Port = -1;
        if (builder.Path.Length > 1) builder.Path = builder.Path.TrimEnd('/');
        return builder.Uri;
    }
}
