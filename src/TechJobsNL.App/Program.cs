using Avalonia;

namespace TechJobsNL.App;

internal static class Program
{
    [STAThread]
    private static void Main(string[] args) =>
        AppBuilder.Configure<DesktopApplication>().UsePlatformDetect().StartWithClassicDesktopLifetime(args);
}
