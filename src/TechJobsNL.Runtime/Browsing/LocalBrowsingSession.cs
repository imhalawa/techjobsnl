using TechJobsNL.Core.Application.Dispatch;

namespace TechJobsNL.Runtime.Browsing;

/// <summary>A local browsing snapshot. Reopen a session to reload data changed by another process.</summary>
public sealed class LocalBrowsingSession : IAsyncDisposable, IQueryDispatcher
{
    private IQueryDispatcher? _queries;

    internal LocalBrowsingSession(IQueryDispatcher queries)
    {
        _queries = queries;
    }

    /// <summary>Gets the shared Core query interface.</summary>
    public IQueryDispatcher Queries => this;

    Task<DispatchResult<TResult>> IQueryDispatcher.QueryAsync<TResult>(
        IQuery<TResult> query, CancellationToken cancellationToken)
    {
        var queries = Volatile.Read(ref _queries);
        ObjectDisposedException.ThrowIf(queries is null, this);
        return queries.QueryAsync(query, cancellationToken);
    }

    /// <summary>Ends this snapshot's lifetime; database handles were released before it became ready.</summary>
    public ValueTask DisposeAsync()
    {
        Interlocked.Exchange(ref _queries, null);
        return ValueTask.CompletedTask;
    }
}
