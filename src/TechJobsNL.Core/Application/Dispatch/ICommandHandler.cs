namespace TechJobsNL.Core.Application.Dispatch;

/// <summary>Handles one explicitly registered command type.</summary>
/// <typeparam name="TCommand">The command to execute.</typeparam>
/// <typeparam name="TResult">The immutable command result.</typeparam>
public interface ICommandHandler<in TCommand, TResult>
    where TCommand : ICommand<TResult>
{
    /// <summary>Executes the command after dispatch validation succeeds.</summary>
    Task<DispatchResult<TResult>> ExecuteAsync(TCommand command, CancellationToken cancellationToken);
}
