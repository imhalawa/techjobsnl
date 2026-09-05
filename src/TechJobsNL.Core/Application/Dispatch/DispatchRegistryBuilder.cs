using System.Collections.Frozen;

namespace TechJobsNL.Core.Application.Dispatch;

/// <summary>Builds command and query dispatchers from explicit, typed handler registrations.</summary>
public sealed class DispatchRegistryBuilder
{
    private readonly Dictionary<Type, object> _commands = [];
    private readonly IDispatchLogger _logger;
    private readonly Dictionary<Type, object> _queries = [];
    private readonly TimeProvider _timeProvider;

    /// <summary>Initializes a builder with the logging and timing services used by all dispatches.</summary>
    public DispatchRegistryBuilder(IDispatchLogger logger, TimeProvider timeProvider)
    {
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
        _timeProvider = timeProvider ?? throw new ArgumentNullException(nameof(timeProvider));
    }

    /// <summary>Registers one command handler and validator.</summary>
    public DispatchRegistryBuilder RegisterCommand<TCommand, TResult>(
        ICommandHandler<TCommand, TResult> handler,
        IRequestValidator<TCommand> validator)
        where TCommand : ICommand<TResult>
    {
        var requestType = typeof(TCommand);
        if (!_commands.TryAdd(requestType, new CommandRegistration<TCommand, TResult>(handler, validator)))
        {
            throw new DuplicateRegistrationException(RequestTypeName(requestType));
        }

        return this;
    }

    /// <summary>Registers one query handler and validator.</summary>
    public DispatchRegistryBuilder RegisterQuery<TQuery, TResult>(
        IQueryHandler<TQuery, TResult> handler,
        IRequestValidator<TQuery> validator)
        where TQuery : IQuery<TResult>
    {
        var requestType = typeof(TQuery);
        if (!_queries.TryAdd(requestType, new QueryRegistration<TQuery, TResult>(handler, validator)))
        {
            throw new DuplicateRegistrationException(RequestTypeName(requestType));
        }

        return this;
    }

    /// <summary>Builds immutable lookup tables for separate command and query dispatch.</summary>
    public Dispatchers Build()
    {
        var registry = new DispatchRegistry(
            _commands.ToFrozenDictionary(),
            _queries.ToFrozenDictionary(),
            _logger,
            _timeProvider);
        return new Dispatchers(registry, registry);
    }

    private static string RequestTypeName(Type requestType) =>
        requestType.FullName ?? requestType.Name;

    private sealed class CommandRegistration<TCommand, TResult> : IDispatchRegistration<TResult>
        where TCommand : ICommand<TResult>
    {
        private readonly ICommandHandler<TCommand, TResult> _handler;
        private readonly IRequestValidator<TCommand> _validator;

        public CommandRegistration(ICommandHandler<TCommand, TResult> handler, IRequestValidator<TCommand> validator)
        {
            _handler = handler ?? throw new ArgumentNullException(nameof(handler));
            _validator = validator ?? throw new ArgumentNullException(nameof(validator));
        }

        public async Task<DispatchResult<TResult>> DispatchAsync(object request, CancellationToken cancellationToken)
        {
            if (request is not TCommand command)
            {
                throw new InvalidOperationException("The explicit command registration did not match the dispatched request.");
            }

            var validation = await _validator.ValidateAsync(command, cancellationToken).ConfigureAwait(false);
            if (validation is ValidationResult.Invalid invalid)
            {
                return new DispatchResult<TResult>.Failure(new DispatchFailure.ValidationFailed(invalid.Code, invalid.Message));
            }

            return await _handler.ExecuteAsync(command, cancellationToken).ConfigureAwait(false);
        }
    }

    private sealed class QueryRegistration<TQuery, TResult> : IDispatchRegistration<TResult>
        where TQuery : IQuery<TResult>
    {
        private readonly IQueryHandler<TQuery, TResult> _handler;
        private readonly IRequestValidator<TQuery> _validator;

        public QueryRegistration(IQueryHandler<TQuery, TResult> handler, IRequestValidator<TQuery> validator)
        {
            _handler = handler ?? throw new ArgumentNullException(nameof(handler));
            _validator = validator ?? throw new ArgumentNullException(nameof(validator));
        }

        public async Task<DispatchResult<TResult>> DispatchAsync(object request, CancellationToken cancellationToken)
        {
            if (request is not TQuery query)
            {
                throw new InvalidOperationException("The explicit query registration did not match the dispatched request.");
            }

            var validation = await _validator.ValidateAsync(query, cancellationToken).ConfigureAwait(false);
            if (validation is ValidationResult.Invalid invalid)
            {
                return new DispatchResult<TResult>.Failure(new DispatchFailure.ValidationFailed(invalid.Code, invalid.Message));
            }

            return await _handler.QueryAsync(query, cancellationToken).ConfigureAwait(false);
        }
    }
}
