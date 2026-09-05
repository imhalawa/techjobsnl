using System.Globalization;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Dapper;
using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Domain.Configuration;

namespace TechJobsNL.Persistence.Sqlite;

public sealed partial class SqliteStore
{
    public async Task PersistCompleteScanAsync(string runId, CompanyConfiguration company, IReadOnlyCollection<ClassifiedVacancy> vacancies, DateTimeOffset startedAt, DateTimeOffset completedAt, CancellationToken cancellationToken)
    {
        var transaction = await _connection.BeginTransactionAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            await _connection.ExecuteAsync(new CommandDefinition("update jobs set is_new = 0 where company_id = @CompanyId;", new { CompanyId = company.Id }, transaction, cancellationToken: cancellationToken)).ConfigureAwait(false);
            const string upsert = """
                insert into jobs (company_id, source_id, title, department, team, employment_type, locations_json, countries_json, job_url, apply_url, description, published_at, raw_payload, content_hash, eligible, eligibility_reason, source_open, is_new, first_seen_at, last_seen_at)
                values (@CompanyId, @SourceId, @Title, @Department, @Team, @EmploymentType, @Locations, @Countries, @JobUrl, @ApplyUrl, @Description, @PublishedAt, @RawPayload, @ContentHash, @Eligible, @Reason, 1, 1, @ObservedAt, @ObservedAt)
                on conflict(company_id, source_id) do update set title=excluded.title, department=excluded.department, team=excluded.team, employment_type=excluded.employment_type, locations_json=excluded.locations_json, countries_json=excluded.countries_json, job_url=excluded.job_url, apply_url=excluded.apply_url, description=excluded.description, published_at=excluded.published_at, raw_payload=excluded.raw_payload, content_hash=excluded.content_hash, eligible=excluded.eligible, eligibility_reason=excluded.eligibility_reason, source_open=1, is_new=jobs.is_new, last_seen_at=excluded.last_seen_at, closed_at=null, reopened_at=case when jobs.source_open=0 then excluded.last_seen_at else jobs.reopened_at end;
                """;
            foreach (var vacancy in vacancies)
            {
                var observed = vacancy.Observed;
                var values = new { CompanyId = company.Id, SourceId = observed.SourceId.Value, observed.Title, observed.Department, observed.Team, EmploymentType = observed.EmploymentType, Locations = JsonSerializer.Serialize(observed.Locations), Countries = JsonSerializer.Serialize(observed.Countries), JobUrl = observed.JobUrl, ApplyUrl = observed.ApplyUrl, observed.Description, PublishedAt = observed.PublishedAt is null ? null : Format(observed.PublishedAt.Value), RawPayload = observed.RawPayload, ContentHash = Hash(observed), Eligible = vacancy.Eligibility.IsEligible ? 1 : 0, Reason = vacancy.Eligibility.Reason, ObservedAt = Format(completedAt) };
                await _connection.ExecuteAsync(new CommandDefinition(upsert, values, transaction, cancellationToken: cancellationToken)).ConfigureAwait(false);
            }

            var observedIds = vacancies.Select(static vacancy => vacancy.Observed.SourceId.Value).ToArray();
            const string close = "update jobs set source_open=0, is_new=0, closed_at=@ClosedAt where company_id=@CompanyId and source_open=1 and source_id not in @ObservedIds;";
            if (observedIds.Length == 0)
                await _connection.ExecuteAsync(new CommandDefinition("update jobs set source_open=0, is_new=0, closed_at=@ClosedAt where company_id=@CompanyId and source_open=1;", new { CompanyId = company.Id, ClosedAt = Format(completedAt) }, transaction, cancellationToken: cancellationToken)).ConfigureAwait(false);
            else
                await _connection.ExecuteAsync(new CommandDefinition(close, new { CompanyId = company.Id, ClosedAt = Format(completedAt), ObservedIds = observedIds }, transaction, cancellationToken: cancellationToken)).ConfigureAwait(false);

            var scan = new { RunId = runId, CompanyId = company.Id, StartedAt = Format(startedAt), CompletedAt = Format(completedAt), Outcome = "complete", ObservedCount = vacancies.Count };
            await _connection.ExecuteAsync(new CommandDefinition("insert into scans (run_id, company_id, started_at, completed_at, outcome, observed_count) values (@RunId,@CompanyId,@StartedAt,@CompletedAt,@Outcome,@ObservedCount);", scan, transaction, cancellationToken: cancellationToken)).ConfigureAwait(false);
            await _connection.ExecuteAsync(new CommandDefinition("update companies set latest_attempted_at=@CompletedAt, latest_successful_at=@CompletedAt, health='healthy', latest_error_kind=null, latest_diagnostic=null where id=@CompanyId;", scan, transaction, cancellationToken: cancellationToken)).ConfigureAwait(false);
            await transaction.CommitAsync(cancellationToken).ConfigureAwait(false);
        }
        finally { await transaction.DisposeAsync().ConfigureAwait(false); }
    }

    public async Task SynchronizeCompaniesAsync(IEnumerable<CompanyConfiguration> companies, CancellationToken cancellationToken)
    {
        var transaction = await _connection.BeginTransactionAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            await _connection.ExecuteAsync(new CommandDefinition("update companies set enabled = 0;", transaction: transaction, cancellationToken: cancellationToken)).ConfigureAwait(false);
            const string sql = """
                insert into companies (id, name, enabled) values (@Id, @Name, @Enabled)
                on conflict(id) do update set name = excluded.name, enabled = excluded.enabled;
                """;
            foreach (var company in companies)
                await _connection.ExecuteAsync(new CommandDefinition(sql, new { Id = company.Id, company.Name, Enabled = company.Enabled ? 1 : 0 }, transaction, cancellationToken: cancellationToken)).ConfigureAwait(false);
            await transaction.CommitAsync(cancellationToken).ConfigureAwait(false);
        }
        finally { await transaction.DisposeAsync().ConfigureAwait(false); }
    }

    public Task RecordCompleteScanAsync(string runId, CompanyId companyId, int observedCount, DateTimeOffset startedAt, DateTimeOffset completedAt, CancellationToken cancellationToken) =>
        RecordScanAsync(runId, companyId, "complete", observedCount, null, null, startedAt, completedAt, cancellationToken);

    public Task RecordFailedScanAsync(string runId, CompanyId companyId, ScanFailure failure, DateTimeOffset startedAt, DateTimeOffset completedAt, CancellationToken cancellationToken) =>
        RecordScanAsync(runId, companyId, "failed", 0, ErrorText(failure.Kind), failure.Diagnostic, startedAt, completedAt, cancellationToken);

    public Task RecordIncompleteScanAsync(string runId, CompanyId companyId, string diagnostic, int observedCount, DateTimeOffset startedAt, DateTimeOffset completedAt, CancellationToken cancellationToken) =>
        RecordScanAsync(runId, companyId, "incomplete", observedCount, "incomplete-results", diagnostic, startedAt, completedAt, cancellationToken);

    public async Task<IReadOnlyList<SourceHealthRecord>> GetSourceHealthAsync(CancellationToken cancellationToken)
    {
        const string sql = "select id Id, name Name, cast(enabled as integer) Enabled, latest_attempted_at LatestAttemptedAt, latest_successful_at LatestSuccessfulAt, health Health, latest_error_kind LatestErrorKind, latest_diagnostic LatestDiagnostic from companies order by name collate nocase, id;";
        var rows = await _connection.QueryAsync<CompanyRow>(new CommandDefinition(sql, cancellationToken: cancellationToken)).ConfigureAwait(false);
        return rows.Select(static row => new SourceHealthRecord(new CompanyId(row.Id), row.Name, row.Enabled != 0,
            ParseTime(row.LatestAttemptedAt), ParseTime(row.LatestSuccessfulAt), ParseHealth(row.Health), ParseError(row.LatestErrorKind), row.LatestDiagnostic)).ToArray();
    }

    public async Task<IReadOnlyList<ScanHistoryRecord>> GetScanHistoryAsync(CancellationToken cancellationToken)
    {
        const string sql = "select s.run_id RunId, s.company_id CompanyId, c.name CompanyName, s.started_at StartedAt, s.completed_at CompletedAt, s.outcome Outcome, s.observed_count ObservedCount, s.error_kind ErrorKind, s.diagnostic Diagnostic from scans s join companies c on c.id = s.company_id order by s.completed_at desc, s.id desc limit 100;";
        var rows = await _connection.QueryAsync<ScanRow>(new CommandDefinition(sql, cancellationToken: cancellationToken)).ConfigureAwait(false);
        return rows.Select(static row => new ScanHistoryRecord(row.RunId, new CompanyId(row.CompanyId), row.CompanyName, ParseRequired(row.StartedAt), ParseRequired(row.CompletedAt), ParseHealth(row.Outcome), checked((int)row.ObservedCount), ParseError(row.ErrorKind), row.Diagnostic)).ToArray();
    }

    private async Task RecordScanAsync(string runId, CompanyId companyId, string outcome, int observedCount, string? errorKind, string? diagnostic, DateTimeOffset startedAt, DateTimeOffset completedAt, CancellationToken cancellationToken)
    {
        const string insert = "insert into scans (run_id, company_id, started_at, completed_at, outcome, observed_count, error_kind, diagnostic) values (@RunId, @CompanyId, @StartedAt, @CompletedAt, @Outcome, @ObservedCount, @ErrorKind, @Diagnostic);";
        var transaction = await _connection.BeginTransactionAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            var values = new { RunId = runId, CompanyId = companyId.Value, StartedAt = Format(startedAt), CompletedAt = Format(completedAt), Outcome = outcome, ObservedCount = observedCount, ErrorKind = errorKind, Diagnostic = diagnostic };
            await _connection.ExecuteAsync(new CommandDefinition(insert, values, transaction, cancellationToken: cancellationToken)).ConfigureAwait(false);
            var update = string.Equals(outcome, "complete", StringComparison.Ordinal)
                ? "update companies set latest_attempted_at=@CompletedAt, latest_successful_at=@CompletedAt, health='healthy', latest_error_kind=null, latest_diagnostic=null where id=@CompanyId;"
                : "update companies set latest_attempted_at=@CompletedAt, health=@Outcome, latest_error_kind=@ErrorKind, latest_diagnostic=@Diagnostic where id=@CompanyId;";
            await _connection.ExecuteAsync(new CommandDefinition(update, values, transaction, cancellationToken: cancellationToken)).ConfigureAwait(false);
            await transaction.CommitAsync(cancellationToken).ConfigureAwait(false);
        }
        finally { await transaction.DisposeAsync().ConfigureAwait(false); }
    }

    private static string Format(DateTimeOffset value) => value.ToUniversalTime().ToString("O", CultureInfo.InvariantCulture);
    private static DateTimeOffset ParseRequired(string value) => DateTimeOffset.Parse(value, CultureInfo.InvariantCulture, DateTimeStyles.RoundtripKind);
    private static DateTimeOffset? ParseTime(string? value) => value is null ? null : ParseRequired(value);
    private static SourceHealth ParseHealth(string value) => value switch { "unknown" => SourceHealth.Unknown, "healthy" or "complete" => SourceHealth.Healthy, "incomplete" => SourceHealth.Incomplete, "failed" => SourceHealth.Failed, _ => throw new InvalidDataException($"Unknown source health '{value}'.") };
    private static SourceErrorKind? ParseError(string? value) => value switch { null => null, "configuration" => SourceErrorKind.Configuration, "transport" => SourceErrorKind.Transport, "timeout" => SourceErrorKind.Timeout, "rate-limit" => SourceErrorKind.RateLimit, "schema" => SourceErrorKind.Schema, "incomplete-results" => SourceErrorKind.IncompleteResults, "browser" => SourceErrorKind.Browser, "storage" => SourceErrorKind.Storage, _ => throw new InvalidDataException($"Unknown source error kind '{value}'.") };
    private static string ErrorText(SourceErrorKind value) => value switch { SourceErrorKind.Configuration => "configuration", SourceErrorKind.Transport => "transport", SourceErrorKind.Timeout => "timeout", SourceErrorKind.RateLimit => "rate-limit", SourceErrorKind.Schema => "schema", SourceErrorKind.IncompleteResults => "incomplete-results", SourceErrorKind.Browser => "browser", SourceErrorKind.Storage => "storage", _ => throw new ArgumentOutOfRangeException(nameof(value)) };
    private static string Hash(ObservedVacancy vacancy) => Convert.ToHexStringLower(SHA256.HashData(Encoding.UTF8.GetBytes(string.Join('\n', vacancy.Title, vacancy.Department, vacancy.Team, vacancy.EmploymentType, string.Join('|', vacancy.Locations), vacancy.JobUrl, vacancy.ApplyUrl, vacancy.Description))));

    private sealed record CompanyRow(string Id, string Name, long Enabled, string? LatestAttemptedAt, string? LatestSuccessfulAt, string Health, string? LatestErrorKind, string? LatestDiagnostic);
    private sealed record ScanRow(string RunId, string CompanyId, string CompanyName, string StartedAt, string CompletedAt, string Outcome, long ObservedCount, string? ErrorKind, string? Diagnostic);
}
