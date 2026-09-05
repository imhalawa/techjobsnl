using CommunityToolkit.Mvvm.ComponentModel;
using TechJobsNL.Core.Vacancies;

namespace TechJobsNL.App.Browsing;

/// <summary>Owns one observable load and cancels/observes it before the window closes.</summary>
public sealed class LocalWindowViewModel : ObservableObject
{
    private readonly CancellationTokenSource _lifetime = new();
    private readonly ILocalWindowSource _source;
    private Task? _loadTask;
    private Task? _closeTask;
    private long _queryVersion;
    private readonly List<Task> _requests = [];
    private string _searchText = "";
    private RetainedVacancy? _selectedVacancy;
    private LocalWindowState _state = new(true, false, [], "Loading retained vacancies…");

    public LocalWindowViewModel(ILocalWindowSource source)
    {
        _source = source;
    }

    public LocalWindowState State { get => _state; private set => SetProperty(ref _state, value); }

    public string SearchText
    {
        get => _searchText;
        set
        {
            if (_closeTask is null && SetProperty(ref _searchText, value ?? ""))
                _loadTask = StartQuery(_searchText);
        }
    }

    public RetainedVacancy? SelectedVacancy
    {
        get => _selectedVacancy;
        set
        {
            if (SetProperty(ref _selectedVacancy, value)) OnPropertyChanged(nameof(HasSelection));
        }
    }

    public bool HasSelection => SelectedVacancy is not null;

    public Task LoadAsync() => _closeTask is not null ? Task.CompletedTask : _loadTask ??= StartQuery("");

    public Task SearchAsync(string search)
    {
        SearchText = search;
        return _closeTask is not null ? Task.CompletedTask : _loadTask ??= StartQuery(search);
    }

    public Task CloseAsync() => _closeTask ??= CloseCoreAsync();

    private Task StartQuery(string search)
    {
        _requests.RemoveAll(static task => task.IsCompleted);
        var task = LoadCoreAsync(search);
        _requests.Add(task);
        return task;
    }

    private async Task LoadCoreAsync(string search)
    {
        var version = ++_queryVersion;
        State = State with { IsLoading = true, IsFailed = false, Message = "Loading retained vacancies…" };
        try
        {
            var result = await _source.LoadAsync(search, _lifetime.Token).ConfigureAwait(true);
            if (_lifetime.IsCancellationRequested || version != _queryVersion)
            {
                return;
            }

            var selectedKey = SelectedVacancy?.Key;
            State = result switch
            {
                LocalWindowLoadResult.Ready ready => new(false, false, ready.Vacancies,
                    ready.Vacancies.IsEmpty ? (string.IsNullOrWhiteSpace(search) ? "No vacancies are stored on this device yet." : "No vacancies match your search.") : "Retained vacancies"),
                LocalWindowLoadResult.Failed failed => FailureState(failed.Message),
                _ => throw new InvalidOperationException("Unknown local load result."),
            };
            if (result is LocalWindowLoadResult.Ready readyResult)
                SelectedVacancy = readyResult.Vacancies.FirstOrDefault(vacancy => vacancy.Key == selectedKey)
                    ?? readyResult.Vacancies.FirstOrDefault();
        }
        catch (OperationCanceledException) when (_lifetime.IsCancellationRequested)
        {
        }
        catch (Exception)
        {
            if (!_lifetime.IsCancellationRequested && version == _queryVersion)
            {
                State = FailureState("Local vacancies could not be loaded. Try searching again.");
            }
        }
    }

    private LocalWindowState FailureState(string message) => State with
    {
        IsLoading = false,
        IsFailed = true,
        Message = message + (State.HasVacancies ? " Previous results are still shown." : ""),
    };

    private async Task CloseCoreAsync()
    {
        await _lifetime.CancelAsync().ConfigureAwait(false);
        await Task.WhenAll(_requests).ConfigureAwait(false);
        await _source.DisposeAsync().ConfigureAwait(false);
        _lifetime.Dispose();
    }
}
