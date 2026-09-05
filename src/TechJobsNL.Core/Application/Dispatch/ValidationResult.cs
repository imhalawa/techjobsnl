namespace TechJobsNL.Core.Application.Dispatch;

/// <summary>Represents the expected result of request validation.</summary>
public abstract record ValidationResult
{
    private ValidationResult()
    {
    }

    /// <summary>Represents a request that may continue to its handler.</summary>
    public sealed record Valid : ValidationResult
    {
        internal Valid()
        {
        }
    }

    /// <summary>Represents a request rejected by typed validation.</summary>
    public sealed record Invalid : ValidationResult
    {
        /// <summary>Initializes a validation failure.</summary>
        public Invalid(string code, string message)
        {
            Code = code;
            Message = message;
        }

        /// <summary>Gets the stable machine-readable validation code.</summary>
        public string Code { get; }

        /// <summary>Gets the user-safe validation message.</summary>
        public string Message { get; }
    }

    /// <summary>Creates a successful validation result.</summary>
    public static ValidationResult ValidResult { get; } = new Valid();
}
