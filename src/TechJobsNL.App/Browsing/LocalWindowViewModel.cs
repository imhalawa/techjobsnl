using CommunityToolkit.Mvvm.ComponentModel;

namespace TechJobsNL.App.Browsing;

/// <summary>Owns one observable load and cancels/observes it before the window closes.</summary>
public sealed class LocalWindowViewModel : ObservableObject
{
    private readonly CancellationTokenSource _lifetime = new();
    private readonly ILocalWindowSource _source;
    private Task? _loadTask;
    private Task? _closeTask;
    private LocalWindowState _state = new(true, false, [], "Loading retained vacancies…");

    public LocalWindowViewModel(ILocalWindowSource source)
    {
        _source = source;
    }

    public LocalWindowState State { get => _state; private set => SetProperty(ref _state, value); }

    public Task LoadAsync() => _closeTask is not null ? Task.CompletedTask : _loadTask ??= LoadCoreAsync();

    public Task CloseAsync() => _closeTask ??= CloseCoreAsync();

    private async Task LoadCoreAsync()
    {
        try
        {
            var result = await _source.LoadAsync(_lifetime.Token).ConfigureAwait(true);
            if (_lifetime.IsCancellationRequested)
            {
                return;
            }

            State = result switch
            {
                LocalWindowLoadResult.Ready ready => new(false, false, ready.Vacancies,
                    ready.Vacancies.IsEmpty ? "No vacancies are stored on this device yet." : "Retained vacancies"),
                LocalWindowLoadResult.Failed failed => new(false, true, [], failed.Message),
                _ => throw new InvalidOperationException("Unknown local load result."),
            };
        }
        catch (OperationCanceledException) when (_lifetime.IsCancellationRequested)
        {
        }
        catch (Exception)
        {
            if (!_lifetime.IsCancellationRequested)
            {
                State = new(false, true, [], "Local vacancies could not be loaded. Close the window and try again.");
            }
        }
    }

    private async Task CloseCoreAsync()
    {
        await _lifetime.CancelAsync().ConfigureAwait(false);
        await (_loadTask ?? Task.CompletedTask).ConfigureAwait(false);
        _lifetime.Dispose();
    }
}
