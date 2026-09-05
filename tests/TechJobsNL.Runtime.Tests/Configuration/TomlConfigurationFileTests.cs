using FluentAssertions;
using TechJobsNL.Runtime.Configuration;
using Tomlyn;
using Tomlyn.Model;

namespace TechJobsNL.Runtime.Tests.Configuration;

public sealed class TomlConfigurationFileTests
{
    [Fact]
    [Trait("TaskId", "V0.1.0-006")]
    public async Task LoadAsync_ArchivedGoldenConfiguration_MaterializesAndValidatesAllCompanies()
    {
        var path = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "..", "archive", "rust", "config.toml"));

        var configuration = await TomlConfigurationFile.LoadAsync(path, TestContext.Current.CancellationToken);

        configuration.SchemaVersion.Should().Be(1);
        configuration.Companies.Should().HaveCount(66);
        configuration.DatabasePath.Should().Be(".data/techjobsnl.sqlite3");
    }

    [Theory]
    [Trait("TaskId", "V0.1.0-006")]
    [InlineData(OperatingSystemKind.Windows, "C:/Users/Alex/AppData/Roaming/techjobsnl/config.toml")]
    [InlineData(OperatingSystemKind.MacOs, "C:/Users/Alex/Library/Application Support/techjobsnl/config.toml")]
    [InlineData(OperatingSystemKind.Linux, "C:/Users/Alex/.config/techjobsnl/config.toml")]
    public void GetConfigurationPath_Platform_UsesCompatibleLocation(OperatingSystemKind operatingSystem, string suffix)
    {
        var result = ConfigurationPaths.GetConfigurationPath(
            operatingSystem,
            "C:/Users/Alex",
            null,
            operatingSystem == OperatingSystemKind.Windows ? "C:/Users/Alex/AppData/Roaming" : null);

        result.Replace('\\', '/').Should().EndWith(suffix);
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-006")]
    public async Task MergeShippedCompaniesAsync_UserChoicesAndUnrelatedKeys_ArePreserved()
    {
        var directory = Directory.CreateTempSubdirectory("techjobsnl-config-");
        try
        {
            var path = Path.Combine(directory.FullName, "config.toml");
            await File.WriteAllTextAsync(path, Existing, TestContext.Current.CancellationToken);

            await TomlConfigurationFile.MergeShippedCompaniesAsync(path, Shipped, TestContext.Current.CancellationToken);

            var root = TomlSerializer.Deserialize<TomlTable>(await File.ReadAllTextAsync(path, TestContext.Current.CancellationToken));
            root.Should().NotBeNull();
            root["custom"]!.Should().Be("retained");
            var companies = (TomlTableArray)root["companies"]!;
            companies.Select(company => company["id"]).Should().Equal("known", "added", "custom");
            companies[0]["enabled"].Should().Be(false);
            companies[0]["name"].Should().Be("Current shipped name");
            companies[2]["extra"].Should().Be("kept");
        }
        finally
        {
            directory.Delete(true);
        }
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-006")]
    public async Task SaveAtomicallyAsync_FailedReplacement_LeavesValidFile()
    {
        var directory = Directory.CreateTempSubdirectory("techjobsnl-config-");
        try
        {
            var path = Path.Combine(directory.FullName, "config.toml");
            await File.WriteAllTextAsync(path, "valid = true", TestContext.Current.CancellationToken);
            Directory.CreateDirectory(path + ".tmp");

            var action = () => TomlConfigurationFile.SaveAtomicallyAsync(path, "invalid", TestContext.Current.CancellationToken);

            await action.Should().ThrowAsync<UnauthorizedAccessException>();
            (await File.ReadAllTextAsync(path, TestContext.Current.CancellationToken)).Should().Be("valid = true");
        }
        finally
        {
            directory.Delete(true);
        }
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-006")]
    public void GetDatabasePath_RelativeValue_UsesConfigurationDirectory()
    {
        var result = ConfigurationPaths.GetDatabasePath("C:/settings/techjobsnl/config.toml", ".data/jobs.db");

        result.Replace('\\', '/').Should().EndWith("C:/settings/techjobsnl/.data/jobs.db");
    }

    private const string Existing = """
        custom = "retained"
        [[companies]]
        id = "known"
        name = "Old name"
        enabled = false
        [[companies]]
        id = "custom"
        name = "Custom"
        enabled = true
        extra = "kept"
        """;

    private const string Shipped = """
        [[companies]]
        id = "known"
        name = "Current shipped name"
        enabled = true
        [[companies]]
        id = "added"
        name = "Added"
        enabled = true
        """;
}
