using TechJobsNL.Core.Domain;

namespace TechJobsNL.Core.Providers;

/// <summary>Represents a safe provider outcome without exposing transport or parser types.</summary>
public abstract record ProviderScanResult
{
    private ProviderScanResult() { }

    /// <summary>Accepts a complete or explicitly incomplete normalized source scan after contract validation.</summary>
    public static ProviderScanResult From(SourceScan scan)
    {
        var observations = scan switch
        {
            SourceScan.Complete complete => complete.Observations,
            SourceScan.Incomplete incomplete => incomplete.Observations,
            _ => throw new InvalidOperationException("Unknown source scan outcome.")
        };
        var sourceIds = new HashSet<SourceId>();
        for (var index = 0; index < observations.Length; index++)
        {
            var observation = observations[index];
            if (string.IsNullOrWhiteSpace(observation.Title)) return Malformed(index, "title is required");
            if (!IsHttps(observation.JobUrl)) return Malformed(index, "job URL must be absolute HTTPS");
            if (!IsHttps(observation.ApplyUrl)) return Malformed(index, "apply URL must be absolute HTTPS");
            if (string.IsNullOrWhiteSpace(observation.RawPayload)) return Malformed(index, "raw evidence is required");
            if (!sourceIds.Add(observation.SourceId)) return Malformed(index, "source ID must be stable and unique within a scan");
        }

        return new Accepted(scan);
    }

    /// <summary>Reports a classified source failure.</summary>
    public static ProviderScanResult Fail(ScanFailure failure) => new Failed(failure);

    /// <summary>Contains a normalized complete or incomplete source scan.</summary>
    public sealed record Accepted(SourceScan Scan) : ProviderScanResult;

    /// <summary>Contains a safe classified failure.</summary>
    public sealed record Failed(ScanFailure Failure) : ProviderScanResult;

    private static Failed Malformed(int index, string reason) =>
        new(new ScanFailure(SourceErrorKind.Schema, $"observation {index}: {reason}"));

    private static bool IsHttps(string value) => Uri.TryCreate(value, UriKind.Absolute, out var uri) &&
        string.Equals(uri.Scheme, Uri.UriSchemeHttps, StringComparison.Ordinal) && !string.IsNullOrWhiteSpace(uri.Host);
}
