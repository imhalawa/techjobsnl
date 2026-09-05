namespace TechJobsNL.Core.Application.Dispatch;

/// <summary>Represents either an immutable response or a typed expected failure.</summary>
/// <typeparam name="TResult">The immutable response type.</typeparam>
public abstract record DispatchResult<TResult>
{
    private DispatchResult()
    {
    }

    /// <summary>Reports a successfully handled request.</summary>
    public sealed record Success : DispatchResult<TResult>
    {
        /// <summary>Initializes a successful response.</summary>
        public Success(TResult value)
        {
            Value = value;
        }

        /// <summary>Gets the immutable response.</summary>
        public TResult Value { get; }
    }

    /// <summary>Reports an expected dispatch failure.</summary>
    public sealed record Failure : DispatchResult<TResult>
    {
        /// <summary>Initializes an expected failure.</summary>
        public Failure(DispatchFailure reason)
        {
            Reason = reason;
        }

        /// <summary>Gets the typed reason the request did not produce a response.</summary>
        public DispatchFailure Reason { get; }
    }
}
