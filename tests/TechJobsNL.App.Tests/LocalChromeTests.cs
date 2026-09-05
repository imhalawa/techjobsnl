using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Avalonia.Interactivity;
using FluentAssertions;
using TechJobsNL.App.Browsing;

namespace TechJobsNL.App.Tests;

[Trait("TaskId", "V0.1.0-079")]
public sealed class LocalChromeTests
{
    [AvaloniaFact]
    public async Task Window_PrototypeCaptions_MaximizeRestoreMinimizeAndClose()
    {
        var model = new LocalWindowViewModel(new EmptySource());
        var window = new LocalWindow(model);
        window.Show();
        await model.LoadAsync();

        window.WindowDecorations.Should().Be(WindowDecorations.None);
        var maximize = window.FindControl<Button>("MaximizeButton");
        Assert.NotNull(maximize);
        maximize.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        window.WindowState.Should().Be(WindowState.Maximized);
        maximize.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        window.WindowState.Should().Be(WindowState.Normal);
        window.FindControl<Button>("MinimizeButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        window.WindowState.Should().Be(WindowState.Minimized);
        window.WindowState = WindowState.Normal;
        window.FindControl<Button>("CloseButton")!.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        await model.CloseAsync();
        window.IsVisible.Should().BeFalse();
    }

    private sealed class EmptySource : ILocalWindowSource
    {
        public Task<LocalWindowLoadResult> LoadAsync(string search, CancellationToken cancellationToken) =>
            Task.FromResult<LocalWindowLoadResult>(new LocalWindowLoadResult.Ready([]));

        public ValueTask DisposeAsync() => ValueTask.CompletedTask;
    }
}
