using Tomlyn;
using Tomlyn.Model;
using TechJobsNL.Core.Domain.Configuration;

namespace TechJobsNL.Runtime.Configuration;

/// <summary>Creates and updates the local TOML document without contacting external systems.</summary>
public static class TomlConfigurationFile
{
    public static async Task<AppConfiguration> LoadAsync(string path, CancellationToken cancellationToken)
    {
        var contents = await File.ReadAllTextAsync(path, cancellationToken).ConfigureAwait(false);
        AppConfiguration configuration;
        try
        {
            configuration = TomlConfigurationMapper.Load(contents);
        }
        catch (Exception exception) when (exception is InvalidCastException or InvalidOperationException or KeyNotFoundException)
        {
            throw new InvalidDataException($"Could not load configuration {Path.GetFullPath(path)}: {exception.Message}", exception);
        }

        if (configuration.Validate() is ConfigurationValidationResult.Invalid invalid)
        {
            throw new InvalidDataException($"Invalid configuration {Path.GetFullPath(path)} at {invalid.Field}: {invalid.Message}");
        }

        return configuration;
    }

    public static async Task EnsureCreatedAsync(
        string path,
        string shippedConfiguration,
        CancellationToken cancellationToken)
    {
        var fullPath = Path.GetFullPath(path);
        Directory.CreateDirectory(Path.GetDirectoryName(fullPath)
            ?? throw new ArgumentException("Configuration path has no parent directory.", nameof(path)));
        try
        {
            using var stream = new FileStream(
                fullPath,
                FileMode.CreateNew,
                FileAccess.Write,
                FileShare.None,
                4096,
                FileOptions.Asynchronous | FileOptions.WriteThrough);
            using var writer = new StreamWriter(stream);
            await writer.WriteAsync(shippedConfiguration.AsMemory(), cancellationToken).ConfigureAwait(false);
            await writer.FlushAsync(cancellationToken).ConfigureAwait(false);
        }
        catch (IOException) when (File.Exists(fullPath))
        {
        }
    }

    public static async Task MergeShippedCompaniesAsync(
        string path,
        string shippedConfiguration,
        CancellationToken cancellationToken)
    {
        var existingText = await File.ReadAllTextAsync(path, cancellationToken).ConfigureAwait(false);
        var existing = Parse(existingText, path);
        var shipped = Parse(shippedConfiguration, "shipped defaults");
        var existingCompanies = Tables(existing, "companies");
        var shippedCompanies = Tables(shipped, "companies");
        var choices = existingCompanies
            .Where(static company => company.TryGetValue("id", out _) && company.TryGetValue("enabled", out _))
            .ToDictionary(static company => (string)company["id"]!, static company => (bool)company["enabled"]!, StringComparer.Ordinal);
        var shippedIds = shippedCompanies.Select(static company => (string)company["id"]!).ToHashSet(StringComparer.Ordinal);
        var merged = new TomlTableArray();
        foreach (var company in shippedCompanies)
        {
            var copy = Clone(company);
            var id = (string)copy["id"]!;
            if (choices.TryGetValue(id, out var enabled))
            {
                copy["enabled"] = enabled;
            }

            merged.Add(copy);
        }

        foreach (var company in existingCompanies.Where(company => !shippedIds.Contains((string)company["id"]!)))
        {
            merged.Add(Clone(company));
        }

        existing["companies"] = merged;
        var updated = TomlSerializer.Serialize(existing);
        if (!string.Equals(updated, existingText, StringComparison.Ordinal))
        {
            await SaveAtomicallyAsync(path, updated, cancellationToken).ConfigureAwait(false);
        }
    }

    public static async Task SaveAtomicallyAsync(string path, string contents, CancellationToken cancellationToken)
    {
        var fullPath = Path.GetFullPath(path);
        var temporaryPath = fullPath + ".tmp";
        await File.WriteAllTextAsync(temporaryPath, contents, cancellationToken).ConfigureAwait(false);
        File.Move(temporaryPath, fullPath, true);
    }

    private static TomlTable Parse(string contents, string path)
    {
        try
        {
            return TomlSerializer.Deserialize<TomlTable>(contents)
                ?? throw new InvalidDataException($"Could not parse configuration {Path.GetFullPath(path)}.");
        }
        catch (Exception exception) when (exception is not InvalidDataException)
        {
            throw new InvalidDataException($"Could not parse configuration {Path.GetFullPath(path)}: {exception.Message}", exception);
        }
    }

    private static TomlTableArray Tables(TomlTable table, string key) =>
        table[key] as TomlTableArray ?? throw new InvalidDataException($"Configuration has no {key} catalog.");

    private static TomlTable Clone(TomlTable table) =>
        TomlSerializer.Deserialize<TomlTable>(TomlSerializer.Serialize(table))
        ?? throw new InvalidDataException("Could not clone a company configuration.");
}
