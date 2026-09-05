using Avalonia;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Themes.Fluent;
using TechJobsNL.App.Browsing;

namespace TechJobsNL.App;

public sealed class DesktopApplication : Application
{
    public override void Initialize() => Styles.Add(new FluentTheme());

    public override void OnFrameworkInitializationCompleted()
    {
        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
        {
            var configurationPath = desktop.Args is { Length: > 0 } ? desktop.Args[0] : null;
            desktop.MainWindow = new LocalWindow(new LocalWindowViewModel(new RuntimeWindowSource(configurationPath)));
        }

        base.OnFrameworkInitializationCompleted();
    }
}
