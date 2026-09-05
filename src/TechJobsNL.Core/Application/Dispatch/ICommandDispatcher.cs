namespace TechJobsNL.Core.Application.Dispatch;

/// <summary>Executes explicitly registered commands.</summary>
public interface ICommandDispatcher
{
    /// <summary>Executes a command through logging, timing, validation, and its handler.</summary>
    Task<DispatchResult<TResult>> ExecuteAsync<TResult>(
        ICommand<TResult> command,
        CancellationToken cancellationToken);
}
