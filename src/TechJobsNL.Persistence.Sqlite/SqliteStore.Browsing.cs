using System.Collections.Immutable;
using Dapper;
using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Ports.Persistence;
using TechJobsNL.Core.Vacancies;

namespace TechJobsNL.Persistence.Sqlite;

public sealed partial class SqliteStore : ILocalVacancyReader
{
    /// <inheritdoc />
    public async Task<VacancyCatalog> ReadVacancyCatalogAsync(CancellationToken cancellationToken)
    {
        const string sql = "select id Id, name Name, enabled Enabled from companies order by id;";
        var companies = await _connection.QueryAsync<BrowsingCompanyRow>(
            new CommandDefinition(sql, cancellationToken: cancellationToken)).ConfigureAwait(false);
        var vacancies = await GetAllVacanciesAsync(cancellationToken).ConfigureAwait(false);
        return new VacancyCatalog(vacancies.ToImmutableArray(), companies.Select(static company =>
            new CompanyView(new CompanyId(company.Id), company.Name, company.Enabled != 0)).ToImmutableArray());
    }

    private sealed record BrowsingCompanyRow(string Id, string Name, long Enabled);
}
