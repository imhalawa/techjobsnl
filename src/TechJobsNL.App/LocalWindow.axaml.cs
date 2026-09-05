using Avalonia.Controls;
using Avalonia.Markup.Xaml;
using Avalonia.Input;
using Avalonia.Interactivity;
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
        this.FindControl<Button>("MinimizeButton")!.Click += (_, _) => WindowState = WindowState.Minimized;
        this.FindControl<Button>("MaximizeButton")!.Click += (_, _) =>
            WindowState = WindowState == WindowState.Maximized ? WindowState.Normal : WindowState.Maximized;
        this.FindControl<Button>("CloseButton")!.Click += (_, _) => Close();
        PropertyChanged += (_, args) =>
        {
            if (args.Property == WindowStateProperty)
                this.FindControl<Button>("MaximizeButton")!.Content = WindowState == WindowState.Maximized ? "❐" : "□";
        };
        this.FindControl<TextBox>("SearchBox")!.AddHandler(KeyDownEvent, OnSearchKeyDown, RoutingStrategies.Tunnel);
    }

    private void OnSearchKeyDown(object? sender, KeyEventArgs args)
    {
        if (args.Key == Key.Escape)
        {
            _model.SearchText = "";
            args.Handled = true;
        }
        else if (args.Key is Key.Up or Key.Down)
        {
            var list = this.FindControl<ListBox>("VacancyList")!;
            if (list.ItemCount > 0)
            {
                list.SelectedIndex = Math.Clamp(list.SelectedIndex + (args.Key == Key.Down ? 1 : -1), 0, list.ItemCount - 1);
                list.ScrollIntoView(list.SelectedIndex);
            }
            args.Handled = true;
        }
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
