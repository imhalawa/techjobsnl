namespace TechJobsNL.Core.Application.Dispatch;

/// <summary>Represents an expected dispatch failure with no exception payload.</summary>
public abstract record DispatchFailure
{
    private DispatchFailure()
    {
    }

    /// <summary>Reports an unregistered request type.</summary>
    public sealed record MissingHandler : DispatchFailure
    {
        /// <summary>Initializes a missing-handler failure.</summary>
        public MissingHandler(string requestType)
        {
            RequestType = requestType;
        }

        /// <summary>Gets the fully qualified missing request type.</summary>
        public string RequestType { get; }
    }

    /// <summary>Reports a typed validation rejection.</summary>
    public sealed record ValidationFailed : DispatchFailure
    {
        /// <summary>Initializes a validation failure.</summary>
        public ValidationFailed(string code, string message)
        {
            Code = code;
            Message = message;
        }

        /// <summary>Gets the stable machine-readable validation code.</summary>
        public string Code { get; }

        /// <summary>Gets the user-safe validation message.</summary>
        public string Message { get; }
    }
}
