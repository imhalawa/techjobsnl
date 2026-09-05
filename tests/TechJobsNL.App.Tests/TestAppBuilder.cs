using Avalonia;
using Avalonia.Headless;

namespace TechJobsNL.App.Tests;

public static class TestAppBuilder
{
    public static AppBuilder BuildAvaloniaApp() =>
        AppBuilder.Configure<DesktopApplication>().UseHeadless(new AvaloniaHeadlessPlatformOptions());
}
