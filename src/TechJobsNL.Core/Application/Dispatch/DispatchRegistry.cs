using System.Collections.Frozen;

namespace TechJobsNL.Core.Application.Dispatch;

internal sealed class DispatchRegistry : ICommandDispatcher, IQueryDispatcher
{
    private readonly FrozenDictionary<Type, object> _commands;
    private readonly IDispatchLogger _logger;
    private readonly FrozenDictionary<Type, object> _queries;
    private readonly TimeProvider _timeProvider;

    public DispatchRegistry(
        FrozenDictionary<Type, object> commands,
        FrozenDictionary<Type, object> queries,
        IDispatchLogger logger,
        TimeProvider timeProvider)
    {
        _commands = commands;
        _queries = queries;
        _logger = logger;
        _timeProvider = timeProvider;
    }

    public Task<DispatchResult<TResult>> ExecuteAsync<TResult>(
        ICommand<TResult> command,
        CancellationToken cancellationToken) =>
        DispatchAsync<TResult>(
            command,
            _commands,
            cancellationToken);

    public Task<DispatchResult<TResult>> QueryAsync<TResult>(
        IQuery<TResult> query,
        CancellationToken cancellationToken) =>
        DispatchAsync<TResult>(
            query,
            _queries,
            cancellationToken);

    private async Task<DispatchResult<TResult>> DispatchAsync<TResult>(
        object request,
        FrozenDictionary<Type, object> registrations,
        CancellationToken cancellationToken)
    {
        var requestType = request.GetType();
        var requestTypeName = requestType.FullName ?? requestType.Name;
        _logger.Started(requestTypeName);
        var startedAt = _timeProvider.GetTimestamp();
        var succeeded = false;

        try
        {
            cancellationToken.ThrowIfCancellationRequested();
            if (!registrations.TryGetValue(requestType, out var registration) ||
                registration is not IDispatchRegistration<TResult> typedRegistration)
            {
                return new DispatchResult<TResult>.Failure(new DispatchFailure.MissingHandler(requestTypeName));
            }

            var result = await typedRegistration.DispatchAsync(request, cancellationToken).ConfigureAwait(false);
            succeeded = result is DispatchResult<TResult>.Success;
            return result;
        }
        finally
        {
            _logger.Completed(requestTypeName, _timeProvider.GetElapsedTime(startedAt), succeeded);
        }
    }
}
