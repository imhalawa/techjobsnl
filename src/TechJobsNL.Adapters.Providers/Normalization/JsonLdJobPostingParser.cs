using System.Net;
using System.Text.Json;
using System.Text.RegularExpressions;
using TechJobsNL.Adapters.Providers.Http;
using TechJobsNL.Core.Domain;

namespace TechJobsNL.Adapters.Providers.Normalization;

/// <summary>Extracts typed JobPosting JSON-LD and normalizes embedded HTML without browser state.</summary>
public static partial class JsonLdJobPostingParser
{
    public static ProviderNormalizationResult<JobPosting> Parse(string html, string source)
    {
        foreach (Match script in JsonLdScript().Matches(html))
        {
            JsonDocument document;
            try { document = JsonDocument.Parse(script.Groups["content"].Value); }
            catch (JsonException exception) { return Fail<JobPosting>($"invalid JSON-LD from {source}: {exception.Message}"); }
            using (document)
            {
                var posting = FindPosting(document.RootElement);
                if (posting is null) continue;
                try
                {
                    var model = posting.Value.Deserialize<JobPosting>();
                    if (model is null) return Fail<JobPosting>($"invalid JobPosting from {source}");
                    if (string.IsNullOrWhiteSpace(HtmlText(model.Description))) return Fail<JobPosting>($"JobPosting from {source} has an empty description");
                    return new ProviderNormalizationResult<JobPosting>.Success(model);
                }
                catch (JsonException exception) { return Fail<JobPosting>($"invalid JobPosting from {source}: {exception.Message}"); }
            }
        }

        return Fail<JobPosting>($"no JobPosting JSON-LD found in {source}");
    }

    public static string HtmlText(string html) => string.Join(' ', WebUtility.HtmlDecode(HtmlTag().Replace(html, " "))
        .Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries));

    private static JsonElement? FindPosting(JsonElement value)
    {
        if (value.ValueKind == JsonValueKind.Object)
        {
            if (value.TryGetProperty("@type", out var type) && IsJobPosting(type)) return value;
            if (value.TryGetProperty("@graph", out var graph)) return FindPosting(graph);
        }
        if (value.ValueKind == JsonValueKind.Array)
            foreach (var item in value.EnumerateArray()) { var result = FindPosting(item); if (result is not null) return result; }
        return null;
    }

    private static bool IsJobPosting(JsonElement type) => type.ValueKind switch
    {
        JsonValueKind.String => string.Equals(type.GetString(), "JobPosting", StringComparison.Ordinal),
        JsonValueKind.Array => type.EnumerateArray().Any(IsJobPosting),
        _ => false
    };

    private static ProviderNormalizationResult<T>.Failure Fail<T>(string message) => new(new ProviderFailure(SourceErrorKind.Schema, message, null, null, false));

    [GeneratedRegex("<script[^>]*type\\s*=\\s*[\"']application/ld\\+json[\"'][^>]*>(?<content>.*?)</script>", RegexOptions.IgnoreCase | RegexOptions.Singleline | RegexOptions.ExplicitCapture, 1000)]
    private static partial Regex JsonLdScript();

    [GeneratedRegex("<[^>]+>", RegexOptions.Singleline, 1000)]
    private static partial Regex HtmlTag();
}
