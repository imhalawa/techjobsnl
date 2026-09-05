namespace TechJobsNL.Core.Application.Dispatch;

/// <summary>Handles one explicitly registered query type.</summary>
/// <typeparam name="TQuery">The query to execute.</typeparam>
/// <typeparam name="TResult">The immutable query result.</typeparam>
public interface IQueryHandler<in TQuery, TResult>
    where TQuery : IQuery<TResult>
{
    /// <summary>Queries canonical state after dispatch validation succeeds.</summary>
    Task<DispatchResult<TResult>> QueryAsync(TQuery query, CancellationToken cancellationToken);
}
