namespace TechJobsNL.Core.Domain;

/// <summary>Identifies a Vacancy at one Company Profile's official source.</summary>
public sealed record VacancyKey
{
    /// <summary>Initializes a stable Company Profile and official source identity pair.</summary>
    public VacancyKey(CompanyId companyId, SourceId sourceId)
    {
        if (!companyId.IsValid)
        {
            throw new ArgumentException("A valid company identifier is required.", nameof(companyId));
        }

        if (!sourceId.IsValid)
        {
            throw new ArgumentException("A valid source identifier is required.", nameof(sourceId));
        }

        CompanyId = companyId;
        SourceId = sourceId;
    }

    /// <summary>Gets the Company Profile identity.</summary>
    public CompanyId CompanyId { get; }

    /// <summary>Gets the official source identity.</summary>
    public SourceId SourceId { get; }
}
