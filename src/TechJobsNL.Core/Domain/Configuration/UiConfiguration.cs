namespace TechJobsNL.Core.Domain.Configuration;

/// <summary>Retains presentation configuration without applying presentation behavior.</summary>
public sealed record UiConfiguration
{
    /// <summary>Initializes a new instance of the <see cref="UiConfiguration"/> class.</summary>
    public UiConfiguration(string theme, bool unicodeIcons, ThemeOverrides themeOverrides)
    {
        Theme = theme;
        UnicodeIcons = unicodeIcons;
        ThemeOverrides = themeOverrides;
    }

    /// <summary>Gets the configured theme name.</summary>
    public string Theme { get; }

    /// <summary>Gets whether Unicode icons are preferred.</summary>
    public bool UnicodeIcons { get; }

    /// <summary>Gets optional semantic color overrides.</summary>
    public ThemeOverrides ThemeOverrides { get; }
}
