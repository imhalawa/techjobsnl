namespace TechJobsNL.Core.Application.Dispatch;

/// <summary>Marks an application request that changes canonical state.</summary>
/// <typeparam name="TResult">The immutable result returned by the command.</typeparam>
public interface ICommand<TResult>;
