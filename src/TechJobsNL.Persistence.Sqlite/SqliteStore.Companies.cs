using System.Globalization;
using Dapper;
using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Domain.Configuration;

namespace TechJobsNL.Persistence.Sqlite;

public sealed partial class SqliteStore
{
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

    private sealed record CompanyRow(string Id, string Name, long Enabled, string? LatestAttemptedAt, string? LatestSuccessfulAt, string Health, string? LatestErrorKind, string? LatestDiagnostic);
    private sealed record ScanRow(string RunId, string CompanyId, string CompanyName, string StartedAt, string CompletedAt, string Outcome, long ObservedCount, string? ErrorKind, string? Diagnostic);
}
