using Avalonia.Controls;
using Avalonia.Markup.Xaml;
using TechJobsNL.App.Browsing;

namespace TechJobsNL.App;

public sealed partial class LocalWindow : Window
{
    private readonly LocalWindowViewModel _model;
    private bool _closing;
    private bool _canClose;

    public LocalWindow() : this(new LocalWindowViewModel(new RuntimeWindowSource(null)))
    {
    }

    public LocalWindow(LocalWindowViewModel model)
    {
        _model = model;
        AvaloniaXamlLoader.Load(this);
        DataContext = model;
        Opened += OnOpened;
        Closing += OnClosing;
    }

    private async void OnOpened(object? sender, EventArgs args) => await _model.LoadAsync().ConfigureAwait(true);

    private async void OnClosing(object? sender, WindowClosingEventArgs args)
    {
        if (_canClose)
        {
            return;
        }

        args.Cancel = true;
        if (_closing)
        {
            return;
        }

        _closing = true;
        await _model.CloseAsync().ConfigureAwait(true);
        _canClose = true;
        Close();
    }
}
