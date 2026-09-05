namespace TechJobsNL.Core.Application.Dispatch;

/// <summary>Records dispatch lifecycle events without coupling Core to a logging implementation.</summary>
public interface IDispatchLogger
{
    /// <summary>Records that dispatch has started.</summary>
    void Started(string requestType);

    /// <summary>Records that dispatch has completed.</summary>
    void Completed(string requestType, TimeSpan elapsed, bool succeeded);
}
