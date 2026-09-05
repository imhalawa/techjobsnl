namespace TechJobsNL.Core.Application.Dispatch;

/// <summary>Thrown when explicit registration assigns more than one handler to a request type.</summary>
public sealed class DuplicateRegistrationException : InvalidOperationException
{
    /// <summary>Initializes a duplicate-registration failure.</summary>
    public DuplicateRegistrationException(string requestType)
        : base($"A handler is already registered for request type '{requestType}'.")
    {
        RequestType = requestType;
    }

    /// <summary>Gets the fully qualified request type registered more than once.</summary>
    public string RequestType { get; }
}
