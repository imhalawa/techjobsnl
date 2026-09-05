using System.Collections.Immutable;
using TechJobsNL.Core.Application.Dispatch;
using TechJobsNL.Core.Vacancies;
using TechJobsNL.Runtime.Browsing;

namespace TechJobsNL.App.Browsing;

public sealed class RuntimeWindowSource : ILocalWindowSource
{
    private readonly string? _configurationPath;
    private Task<LocalBrowsingOpenResult>? _opening;

    public RuntimeWindowSource(string? configurationPath)
    {
        _configurationPath = configurationPath;
    }

    public async ValueTask DisposeAsync()
    {
        if (_opening is null) return;
        LocalBrowsingOpenResult result;
        try
        {
            result = await _opening.ConfigureAwait(false);
        }
        catch (Exception) when (_opening.IsCanceled || _opening.IsFaulted)
        {
            // The loading boundary already observes startup failures; no session was returned to dispose.
            return;
        }
        if (result is LocalBrowsingOpenResult.Opened opened)
        {
            await opened.Session.DisposeAsync().ConfigureAwait(false);
        }
    }

    public async Task<LocalWindowLoadResult> LoadAsync(string search, CancellationToken cancellationToken)
    {
        _opening ??= _configurationPath is null
            ? LocalBrowsingRuntime.OpenDefaultAsync(cancellationToken)
            : LocalBrowsingRuntime.OpenAsync(_configurationPath, cancellationToken);
        var result = await _opening.ConfigureAwait(false);
        if (result is LocalBrowsingOpenResult.Failed failed)
        {
            return new LocalWindowLoadResult.Failed(failed.Message);
        }

        var opened = (LocalBrowsingOpenResult.Opened)result;
        var query = await Task.Run(() => opened.Session.Queries.QueryAsync(new BrowseVacancies(search), cancellationToken), cancellationToken)
            .ConfigureAwait(false);
        return query is DispatchResult<ImmutableArray<RetainedVacancy>>.Success success
            ? new LocalWindowLoadResult.Ready(success.Value)
            : new LocalWindowLoadResult.Failed("Local vacancies could not be read.");
    }
}
