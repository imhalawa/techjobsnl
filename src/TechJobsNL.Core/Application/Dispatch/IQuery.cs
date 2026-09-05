namespace TechJobsNL.Core.Application.Dispatch;

/// <summary>Marks an application request that reads canonical state.</summary>
/// <typeparam name="TResult">The immutable result returned by the query.</typeparam>
public interface IQuery<TResult>;
