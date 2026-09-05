namespace TechJobsNL.Core.Domain;

/// <summary>Identifies a Vacancy at an official source.</summary>
public readonly record struct SourceId
{
    /// <summary>Initializes an official source identifier.</summary>
    public SourceId(string value)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(value);
        Value = value;
    }

    /// <summary>Gets the stable identifier value.</summary>
    public string Value { get; }

    /// <summary>Gets whether this value was initialized through a valid identifier boundary.</summary>
    public bool IsValid => !string.IsNullOrWhiteSpace(Value);

    /// <inheritdoc />
    public override string ToString() => Value;
}
