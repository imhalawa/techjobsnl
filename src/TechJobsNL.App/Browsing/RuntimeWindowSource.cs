using System.Collections.Immutable;
using TechJobsNL.Core.Application.Dispatch;
using TechJobsNL.Core.Vacancies;
using TechJobsNL.Runtime.Browsing;

namespace TechJobsNL.App.Browsing;

public sealed class RuntimeWindowSource : ILocalWindowSource
{
    private readonly string? _configurationPath;

    public RuntimeWindowSource(string? configurationPath)
    {
        _configurationPath = configurationPath;
    }

    public async Task<LocalWindowLoadResult> LoadAsync(CancellationToken cancellationToken)
    {
        var result = await (_configurationPath is null
            ? LocalBrowsingRuntime.OpenDefaultAsync(cancellationToken)
            : LocalBrowsingRuntime.OpenAsync(_configurationPath, cancellationToken)).ConfigureAwait(false);
        if (result is LocalBrowsingOpenResult.Failed failed)
        {
            return new LocalWindowLoadResult.Failed(failed.Message);
        }

        var opened = (LocalBrowsingOpenResult.Opened)result;
        var session = opened.Session;
        await using (session.ConfigureAwait(false))
        {
            var query = await session.Queries.QueryAsync(new BrowseVacancies(""), cancellationToken).ConfigureAwait(false);
            return query is DispatchResult<ImmutableArray<RetainedVacancy>>.Success success
                ? new LocalWindowLoadResult.Ready(success.Value)
                : new LocalWindowLoadResult.Failed("Local vacancies could not be read.");
        }
    }
}
