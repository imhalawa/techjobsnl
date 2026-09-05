namespace TechJobsNL.Runtime.Configuration;

/// <summary>Resolves the compatible local configuration and database locations.</summary>
public static class ConfigurationPaths
{
    public static string GetConfigurationPath(
        OperatingSystemKind operatingSystem,
        string? homeDirectory,
        string? xdgConfigurationDirectory,
        string? applicationDataDirectory)
    {
        var baseDirectory = operatingSystem switch
        {
            OperatingSystemKind.Windows => applicationDataDirectory ?? CombineRequired(homeDirectory, "AppData", "Roaming"),
            OperatingSystemKind.MacOs => CombineRequired(homeDirectory, "Library", "Application Support"),
            OperatingSystemKind.Linux => xdgConfigurationDirectory ?? CombineRequired(homeDirectory, ".config"),
            _ => throw new ArgumentOutOfRangeException(nameof(operatingSystem))
        };

        return Path.GetFullPath(Path.Combine(baseDirectory, "techjobsnl", "config.toml"));
    }

    public static string GetDatabasePath(string configurationPath, string configuredDatabasePath)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(configurationPath);
        ArgumentException.ThrowIfNullOrWhiteSpace(configuredDatabasePath);
        var configurationDirectory = Path.GetDirectoryName(Path.GetFullPath(configurationPath))
            ?? throw new ArgumentException("Configuration path has no parent directory.", nameof(configurationPath));
        return Path.GetFullPath(Path.IsPathFullyQualified(configuredDatabasePath)
            ? configuredDatabasePath
            : Path.Combine(configurationDirectory, configuredDatabasePath));
    }

    private static string CombineRequired(string? root, params string[] parts)
    {
        if (string.IsNullOrWhiteSpace(root))
        {
            throw new DirectoryNotFoundException("User configuration directory is unavailable.");
        }

        return Path.Combine([root, .. parts]);
    }
}
