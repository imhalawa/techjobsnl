namespace TechJobsNL.App.Browsing;

/// <summary>The Runtime boundary used to load the desktop's local vacancy snapshot.</summary>
public interface ILocalWindowSource : IAsyncDisposable
{
    Task<LocalWindowLoadResult> LoadAsync(string search, CancellationToken cancellationToken);
}
