using System.Xml.Linq;

namespace TechJobsNL.Core.Tests;

public sealed class ProjectGraphTests
{
    [Fact]
    [Trait("TaskId", "V0.1.0-001")]
    public void ProjectGraph_ArchitectureDependencies_AreExact()
    {
        var repositoryRoot = FindRepositoryRoot();
        var expectedReferences = new Dictionary<string, string[]>(StringComparer.Ordinal)
        {
            ["src/TechJobsNL.Core/TechJobsNL.Core.csproj"] = [],
            ["src/TechJobsNL.Runtime/TechJobsNL.Runtime.csproj"] = ["../TechJobsNL.Core/TechJobsNL.Core.csproj"],
            ["src/TechJobsNL.App/TechJobsNL.App.csproj"] = ["../TechJobsNL.Runtime/TechJobsNL.Runtime.csproj"],
            ["src/TechJobsNL.Tui/TechJobsNL.Tui.csproj"] = ["../TechJobsNL.Runtime/TechJobsNL.Runtime.csproj"],
            ["src/TechJobsNL.Adapters.Providers/TechJobsNL.Adapters.Providers.csproj"] = ["../TechJobsNL.Core/TechJobsNL.Core.csproj"],
            ["src/TechJobsNL.Adapters.Analytics.Local/TechJobsNL.Adapters.Analytics.Local.csproj"] = ["../TechJobsNL.Core/TechJobsNL.Core.csproj"],
            ["src/TechJobsNL.Adapters.AiExperience.DeepSeek/TechJobsNL.Adapters.AiExperience.DeepSeek.csproj"] = ["../TechJobsNL.Core/TechJobsNL.Core.csproj"],
            ["src/TechJobsNL.Persistence.Sqlite/TechJobsNL.Persistence.Sqlite.csproj"] = ["../TechJobsNL.Core/TechJobsNL.Core.csproj"],
            ["src/TechJobsNL.Adapters.Platform/TechJobsNL.Adapters.Platform.csproj"] = ["../TechJobsNL.Core/TechJobsNL.Core.csproj"]
        };

        foreach (var (projectPath, expectedProjectReferences) in expectedReferences)
        {
            var document = XDocument.Load(Path.Combine(repositoryRoot.FullName, projectPath));
            var actualProjectReferences = document.Descendants("ProjectReference")
                .Select(reference => reference.Attribute("Include")?.Value)
                .OfType<string>()
                .Order(StringComparer.Ordinal)
                .ToArray();

            Assert.Equal(expectedProjectReferences.Order(StringComparer.Ordinal), actualProjectReferences);
        }
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-001")]
    public void ProjectGraph_Clients_AreEmptyExecutableShells()
    {
        var repositoryRoot = FindRepositoryRoot();

        AssertEmptyShell(repositoryRoot, "TechJobsNL.App");
        AssertEmptyShell(repositoryRoot, "TechJobsNL.Tui");
    }

    private static DirectoryInfo FindRepositoryRoot()
    {
        DirectoryInfo? directory = new(AppContext.BaseDirectory);

        while (directory is not null)
        {
            if (File.Exists(Path.Combine(directory.FullName, "TechJobsNL.slnx")))
            {
                return directory;
            }

            directory = directory.Parent;
        }

        throw new DirectoryNotFoundException("Could not find the repository root.");
    }

    private static void AssertEmptyShell(DirectoryInfo repositoryRoot, string projectName)
    {
        var projectDirectory = Path.Combine(repositoryRoot.FullName, "src", projectName);
        var sourceFiles = Directory.GetFiles(projectDirectory, "*.cs", SearchOption.TopDirectoryOnly)
            .Select(filePath => Path.GetFileName(filePath) ?? string.Empty)
            .Order(StringComparer.Ordinal)
            .ToArray();
        var project = XDocument.Load(Path.Combine(projectDirectory, $"{projectName}.csproj"));

        Assert.Equal(["Program.cs"], sourceFiles);
        Assert.Equal("Exe", project.Root?.Element("PropertyGroup")?.Element("OutputType")?.Value);
    }
}
