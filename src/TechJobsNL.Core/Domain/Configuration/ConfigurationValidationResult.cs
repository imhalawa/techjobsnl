namespace TechJobsNL.Core.Domain.Configuration;

/// <summary>Represents the result of deterministic configuration validation.</summary>
public abstract record ConfigurationValidationResult
{
    private ConfigurationValidationResult()
    {
    }

    /// <summary>Indicates that validation completed successfully.</summary>
    public sealed record Valid : ConfigurationValidationResult;

    /// <summary>Describes the first invalid configuration field in Rust-compatible order.</summary>
    public sealed record Invalid : ConfigurationValidationResult
    {
        /// <summary>Initializes a new instance of the <see cref="Invalid"/> class.</summary>
        public Invalid(string field, string message)
        {
            Field = field;
            Message = message;
        }

        /// <summary>Gets the path of the invalid field.</summary>
        public string Field { get; }

        /// <summary>Gets the reason the field is invalid.</summary>
        public string Message { get; }

        /// <inheritdoc />
        public override string ToString() => $"invalid configuration at {Field}: {Message}";
    }
}
