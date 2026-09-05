using System.Collections.Immutable;

namespace TechJobsNL.Core.Domain.Configuration;

/// <summary>Represents one raw company profile from configuration.</summary>
public sealed record CompanyConfiguration
{
    /// <summary>Initializes a new instance of the <see cref="CompanyConfiguration"/> class.</summary>
    public CompanyConfiguration(
        string id,
        string name,
        string industry,
        string scale,
        bool enabled,
        ImmutableDictionary<string, string> locationCountryOverrides,
        SourceConfiguration source)
    {
        Id = id;
        Name = name;
        Industry = industry;
        Scale = scale;
        Enabled = enabled;
        LocationCountryOverrides = locationCountryOverrides;
        Source = source;
    }

    /// <summary>Gets the Rust default for omitted display metadata.</summary>
    public const string UnknownMetadata = "Unknown";

    /// <summary>Gets the raw stable company id, before validation.</summary>
    public string Id { get; }

    /// <summary>Gets the display name.</summary>
    public string Name { get; }

    /// <summary>Gets the retained industry metadata.</summary>
    public string Industry { get; }

    /// <summary>Gets the retained company-scale metadata.</summary>
    public string Scale { get; }

    /// <summary>Gets whether the company is enabled for scans.</summary>
    public bool Enabled { get; }

    /// <summary>Gets exact raw location-label country overrides.</summary>
    public ImmutableDictionary<string, string> LocationCountryOverrides { get; }

    /// <summary>Gets the configured source strategy.</summary>
    public SourceConfiguration Source { get; }
}
