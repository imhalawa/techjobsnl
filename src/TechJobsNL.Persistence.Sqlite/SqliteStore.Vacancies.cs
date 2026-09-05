using System.Collections.Immutable;
using System.Text.Json;
using Dapper;
using TechJobsNL.Core.Domain;

namespace TechJobsNL.Persistence.Sqlite;

public sealed partial class SqliteStore
{
    public async Task<IReadOnlyList<VacancyRecord>> GetAllVacanciesAsync(CancellationToken cancellationToken)
    {
        const string sql = """
            select company_id CompanyId, source_id SourceId, title Title, department Department, team Team, employment_type EmploymentType,
              locations_json LocationsJson, countries_json CountriesJson, job_url JobUrl, apply_url ApplyUrl, description Description, raw_payload RawPayload,
              published_at PublishedAt, eligible Eligible, eligibility_reason EligibilityReason, source_open SourceOpen, is_new IsNew,
              first_seen_at FirstSeenAt, last_seen_at LastSeenAt, closed_at ClosedAt, reopened_at ReopenedAt, applied_at AppliedAt
            from jobs order by last_seen_at desc, company_id, source_id;
            """;
        var rows = await _connection.QueryAsync<VacancyRow>(new CommandDefinition(sql, cancellationToken: cancellationToken)).ConfigureAwait(false);
        return rows.Select(static row => new VacancyRecord(
            new VacancyKey(new CompanyId(row.CompanyId), new SourceId(row.SourceId)),
            new ClassifiedVacancy(new ObservedVacancy(new SourceId(row.SourceId), row.Title, row.Department, row.Team, row.EmploymentType,
                ParseStrings(row.LocationsJson), ParseStrings(row.CountriesJson), row.JobUrl, row.ApplyUrl, row.Description, row.RawPayload, ParseTime(row.PublishedAt)),
                new TechJobsNL.Core.Domain.Eligibility(row.Eligible != 0, row.EligibilityReason)),
            row.SourceOpen != 0, row.IsNew != 0, ParseRequired(row.FirstSeenAt), ParseRequired(row.LastSeenAt), ParseTime(row.ClosedAt), ParseTime(row.ReopenedAt), ParseTime(row.AppliedAt))).ToArray();
    }

    private static ImmutableArray<string> ParseStrings(string json) => JsonSerializer.Deserialize<ImmutableArray<string>>(json, CompatibleJson);
    private sealed record VacancyRow(string CompanyId, string SourceId, string Title, string? Department, string? Team, string? EmploymentType, string LocationsJson, string CountriesJson, string JobUrl, string ApplyUrl, string Description, string RawPayload, string? PublishedAt, long Eligible, string EligibilityReason, long SourceOpen, long IsNew, string FirstSeenAt, string LastSeenAt, string? ClosedAt, string? ReopenedAt, string? AppliedAt);
}
