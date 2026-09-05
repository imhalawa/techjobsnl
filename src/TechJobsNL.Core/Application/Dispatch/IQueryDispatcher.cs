namespace TechJobsNL.Core.Application.Dispatch;

/// <summary>Queries through explicitly registered handlers.</summary>
public interface IQueryDispatcher
{
    /// <summary>Queries through logging, timing, validation, and its handler.</summary>
    Task<DispatchResult<TResult>> QueryAsync<TResult>(
        IQuery<TResult> query,
        CancellationToken cancellationToken);
}
