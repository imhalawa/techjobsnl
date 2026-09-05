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
            ["src/TechJobsNL.Runtime/TechJobsNL.Runtime.csproj"] =
            [
                "../TechJobsNL.Adapters.AiExperience.DeepSeek/TechJobsNL.Adapters.AiExperience.DeepSeek.csproj",
                "../TechJobsNL.Adapters.Analytics.Local/TechJobsNL.Adapters.Analytics.Local.csproj",
                "../TechJobsNL.Adapters.Platform/TechJobsNL.Adapters.Platform.csproj",
                "../TechJobsNL.Adapters.Providers/TechJobsNL.Adapters.Providers.csproj",
                "../TechJobsNL.Core/TechJobsNL.Core.csproj",
                "../TechJobsNL.Persistence.Sqlite/TechJobsNL.Persistence.Sqlite.csproj"
            ],
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
    [Trait("TaskId", "V0.1.0-002")]
    public void ProjectGraph_ForbiddenReferences_AreRejected()
    {
        var forbidden = new Dictionary<string, string[]>(StringComparer.Ordinal)
        {
            ["TechJobsNL.Core"] = ["TechJobsNL.Runtime", "TechJobsNL.App", "TechJobsNL.Tui", "Dapper", "Avalonia"],
            ["TechJobsNL.App"] = ["TechJobsNL.Tui"],
            ["TechJobsNL.Tui"] = ["TechJobsNL.App"],
            ["TechJobsNL.Adapters.Providers"] = ["TechJobsNL.Adapters.Analytics.Local"]
        };

        foreach (var (owner, references) in forbidden)
        {
            foreach (var referenced in references)
            {
                var fixture = $"<Project><ItemGroup><ProjectReference Include=\"{referenced}\" /></ItemGroup></Project>";
                Assert.False(IsAllowedReference(owner, referenced, fixture), $"Forbidden fixture was accepted: {owner} -> {referenced}");
            }
        }
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-002")]
    public void Repository_ProjectGraph_AndConventions_AreEnforced()
    {
        var root = FindRepositoryRoot();
        var props = XDocument.Load(Path.Combine(root.FullName, "Directory.Build.props"));
        var propertyGroup = props.Root?.Element("PropertyGroup");

        Assert.Equal("net10.0", propertyGroup?.Element("TargetFramework")?.Value);
        Assert.Equal("14.0", propertyGroup?.Element("LangVersion")?.Value);
        Assert.Equal("enable", propertyGroup?.Element("Nullable")?.Value);
        Assert.Equal("true", propertyGroup?.Element("TreatWarningsAsErrors")?.Value);
        Assert.Equal("true", propertyGroup?.Element("Deterministic")?.Value);
        Assert.True(File.Exists(Path.Combine(root.FullName, ".editorconfig")));

        foreach (var sourceFile in Directory.EnumerateFiles(Path.Combine(root.FullName, "src"), "*.cs", SearchOption.AllDirectories))
        {
            var relative = Path.GetRelativePath(root.FullName, sourceFile).Replace('\\', '/');
            if (relative.Split('/').Any(part => part is "bin" or "obj"))
            {
                continue;
            }

            var project = relative.Split('/')[1];
            var expectedNamespace = project.Replace(".", ".");
            var text = File.ReadAllText(sourceFile);
            Assert.Contains($"namespace {expectedNamespace};", text, StringComparison.Ordinal);
        }
    }

    private static bool IsAllowedReference(string owner, string referenced, string projectXml)
    {
        var document = XDocument.Parse(projectXml);
        var actual = document.Descendants("ProjectReference")
            .Select(element => element.Attribute("Include")?.Value ?? string.Empty)
            .Concat(document.Descendants("PackageReference").Select(element => element.Attribute("Include")?.Value ?? string.Empty));
        return actual.Contains(referenced, StringComparer.Ordinal) && owner switch
        {
            "TechJobsNL.Core" => referenced is "TechJobsNL.Core",
            "TechJobsNL.App" or "TechJobsNL.Tui" => referenced is "TechJobsNL.Runtime",
            _ => referenced is "TechJobsNL.Core"
        };
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
