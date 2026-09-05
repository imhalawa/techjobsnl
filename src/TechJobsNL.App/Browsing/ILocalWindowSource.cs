namespace TechJobsNL.App.Browsing;

/// <summary>The Runtime boundary used to load the desktop's local vacancy snapshot.</summary>
public interface ILocalWindowSource
{
    Task<LocalWindowLoadResult> LoadAsync(CancellationToken cancellationToken);
}
