namespace TechJobsNL.Core.Domain;

/// <summary>Identifies a Company Profile.</summary>
public readonly record struct CompanyId
{
    /// <summary>Initializes a Company Profile identifier.</summary>
    public CompanyId(string value)
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
