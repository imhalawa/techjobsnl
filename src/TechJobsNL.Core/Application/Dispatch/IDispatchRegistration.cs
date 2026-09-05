namespace TechJobsNL.Core.Application.Dispatch;

internal interface IDispatchRegistration<TResult>
{
    Task<DispatchResult<TResult>> DispatchAsync(object request, CancellationToken cancellationToken);
}
