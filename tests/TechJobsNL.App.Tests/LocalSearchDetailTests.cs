using Avalonia.Headless.XUnit;
using Avalonia.Headless;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Input.Raw;
using FluentAssertions;
using Microsoft.Data.Sqlite;
using TechJobsNL.App.Browsing;
using TechJobsNL.Runtime.Browsing;
using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Vacancies;

namespace TechJobsNL.App.Tests;

[Trait("TaskId", "V0.1.0-078")]
public sealed class LocalSearchDetailTests
{
    [AvaloniaFact]
    public async Task Window_ArrowKeysInSearch_MoveSelectionWithoutLeavingSearch()
    {
        await using var fixture = await LocalFixture.CreateAsync();
        var model = new LocalWindowViewModel(new RuntimeWindowSource(fixture.ConfigurationPath));
        var window = new LocalWindow(model);
        window.Show();
        await model.LoadAsync();
        var search = window.FindControl<TextBox>("SearchBox")!;
        search.Focus(NavigationMethod.Tab);
        window.KeyPress(Key.Down, RawInputModifiers.None, PhysicalKey.ArrowDown, null);
        window.KeyRelease(Key.Down, RawInputModifiers.None, PhysicalKey.ArrowDown, null);

        search.IsFocused.Should().BeTrue();
        window.FindControl<TextBlock>("DetailTitle")!.Text.Should().Be("Backend Engineer");
        window.Close();
        await model.CloseAsync();
    }

    [AvaloniaFact]
    public async Task Window_SearchAndSelection_ShowRetainedDetailsAndClearPredictably()
    {
        await using var fixture = await LocalFixture.CreateAsync();
        var model = new LocalWindowViewModel(new RuntimeWindowSource(fixture.ConfigurationPath));
        var window = new LocalWindow(model);
        window.Show();
        await model.LoadAsync();
        var search = window.FindControl<TextBox>("SearchBox");
        Assert.NotNull(search);
        search.Text = "beta";
        await model.SearchAsync("beta");
        window.UpdateLayout();

        window.FindControl<TextBlock>("DetailTitle")!.Text.Should().Be("Backend Engineer");
        window.FindControl<TextBlock>("DetailCompany")!.Text.Should().Be("Beta Labs");
        window.FindControl<SelectableTextBlock>("DetailDescription")!.Text.Should().Be("Build services.");
        window.FindControl<SelectableTextBlock>("DetailUrl")!.Text.Should().Be("https://example.test/jobs/two");

        search.Focus(NavigationMethod.Tab);
        window.KeyPress(Key.Escape, RawInputModifiers.None, PhysicalKey.Escape, null);
        window.KeyRelease(Key.Escape, RawInputModifiers.None, PhysicalKey.Escape, null);
        search.Text.Should().BeEmpty();
        await model.SearchAsync("");
        model.State.Vacancies.Should().HaveCount(2);
        window.FindControl<TextBlock>("DetailTitle")!.Text.Should().Be("Backend Engineer");

        search.Text = "no matches";
        await model.SearchAsync("no matches");
        window.FindControl<ScrollViewer>("DetailPane")!.IsVisible.Should().BeFalse();
        window.FindControl<TextBlock>("EmptySelection")!.IsVisible.Should().BeTrue();
        window.Close();
        await model.CloseAsync();
    }

    [AvaloniaFact]
    public async Task Close_CancelledRuntimeStartup_DoesNotRethrowObservedCancellation()
    {
        using var cancellation = new CancellationTokenSource();
        await cancellation.CancelAsync();
        var source = new RuntimeWindowSource("missing.toml");
        var load = () => source.LoadAsync("", cancellation.Token);
        await load.Should().ThrowAsync<OperationCanceledException>();
        await source.DisposeAsync();
    }

    [AvaloniaFact]
    public async Task Search_OpenedSnapshot_DoesNotReopenSettingsOrStorage()
    {
        await using var fixture = await LocalFixture.CreateAsync();
        var model = new LocalWindowViewModel(new RuntimeWindowSource(fixture.ConfigurationPath));
        await model.LoadAsync();
        File.Move(fixture.ConfigurationPath, fixture.ConfigurationPath + ".held");

        await model.SearchAsync("platform");

        model.State.IsFailed.Should().BeFalse();
        model.State.Vacancies.Should().ContainSingle().Which.Title.Should().Be("Platform Engineer");
        await model.CloseAsync();
    }

    [AvaloniaFact]
    public async Task Close_OverlappingSearches_ObservesEveryRequestBeforeReleasingSource()
    {
        var source = new ControlledSource();
        var model = new LocalWindowViewModel(source);
        var older = model.LoadAsync();
        var latest = model.SearchAsync("newer");
        source.Requests[1].SetResult(new LocalWindowLoadResult.Ready([]));
        await latest;

        var closing = model.CloseAsync();
        closing.IsCompleted.Should().BeFalse();
        source.Disposed.Should().BeFalse();
        source.Requests[0].SetException(new IOException("late fault"));
        await closing;
        await older;
        source.Disposed.Should().BeTrue();
    }

    [AvaloniaFact]
    public async Task Search_DelayedResults_UsesLatestInputAndPreservesTrustedRowsOnFailure()
    {
        var source = new ControlledSource();
        var model = new LocalWindowViewModel(source);
        var window = new LocalWindow(model);
        window.Show();
        var initial = model.LoadAsync();
        source.Requests[0].SetResult(new LocalWindowLoadResult.Ready([Vacancy("initial")]));
        await initial;

        var older = model.SearchAsync("older");
        var latest = model.SearchAsync("latest");
        model.State.IsLoading.Should().BeTrue();
        model.State.Vacancies.Should().ContainSingle().Which.Title.Should().Be("initial");
        source.Requests[2].SetResult(new LocalWindowLoadResult.Ready([Vacancy("latest")]));
        await latest;
        source.Requests[1].SetResult(new LocalWindowLoadResult.Ready([Vacancy("older")]));
        await older;
        model.State.Vacancies.Should().ContainSingle().Which.Title.Should().Be("latest");

        var failed = model.SearchAsync("failing");
        source.Requests[3].SetException(new IOException("secret internal path"));
        await failed;
        model.State.IsFailed.Should().BeTrue();
        model.State.IsLoading.Should().BeFalse();
        model.State.Vacancies.Should().ContainSingle().Which.Title.Should().Be("latest");
        model.State.Message.Should().NotContain("secret");
        model.State.Message.Should().Contain("Previous results");
        window.FindControl<TextBlock>("ErrorMessage")!.IsVisible.Should().BeTrue();
        window.FindControl<TextBlock>("DetailTitle")!.Text.Should().Be("latest");
        window.Close();
        await model.CloseAsync();
    }

    private static RetainedVacancy Vacancy(string title) => new(
        new VacancyKey(new CompanyId("alpha"), new SourceId(title)), "Alpha", title, [], "Description",
        "https://example.test/job", "https://example.test/apply", true, DateTimeOffset.UnixEpoch);

    private sealed class ControlledSource : ILocalWindowSource
    {
        public bool Disposed { get; private set; }
        public List<TaskCompletionSource<LocalWindowLoadResult>> Requests { get; } = [];

        public ValueTask DisposeAsync()
        {
            Disposed = true;
            return ValueTask.CompletedTask;
        }

        public Task<LocalWindowLoadResult> LoadAsync(string search, CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<LocalWindowLoadResult>(TaskCreationOptions.RunContinuationsAsynchronously);
            Requests.Add(completion);
            return completion.Task;
        }
    }

    [AvaloniaFact]
    public async Task Search_SharedQuery_MatchesTitleAndCompanyButNotDescription_AndClears()
    {
        await using var fixture = await LocalFixture.CreateAsync();
        var model = new LocalWindowViewModel(new RuntimeWindowSource(fixture.ConfigurationPath));
        await model.LoadAsync();
        model.State.Vacancies.Should().HaveCount(2);

        await model.SearchAsync("  PLATFORM  ");
        model.State.Vacancies.Should().ContainSingle().Which.Title.Should().Be("Platform Engineer");
        await model.SearchAsync("beta labs");
        model.State.Vacancies.Should().ContainSingle().Which.CompanyName.Should().Be("Beta Labs");
        await model.SearchAsync("ledgers");
        model.State.Vacancies.Should().BeEmpty();
        await model.SearchAsync("");
        model.State.Vacancies.Should().HaveCount(2);
        await model.CloseAsync();
    }

    private sealed class LocalFixture : IAsyncDisposable
    {
        private readonly DirectoryInfo _directory;

        private LocalFixture(DirectoryInfo directory) { _directory = directory; }

        public string ConfigurationPath => Path.Combine(_directory.FullName, "config.toml");

        public static async Task<LocalFixture> CreateAsync()
        {
            var fixture = new LocalFixture(Directory.CreateTempSubdirectory("techjobsnl-search-"));
            var token = TestContext.Current.CancellationToken;
            var result = await LocalBrowsingRuntime.CreateAndOpenAsync(fixture.ConfigurationPath, token).ConfigureAwait(false);
            var opened = result.Should().BeOfType<LocalBrowsingOpenResult.Opened>().Which;
            await opened.Session.DisposeAsync().ConfigureAwait(false);
            using var connection = new SqliteConnection($"Data Source={Path.Combine(fixture._directory.FullName, ".data", "techjobsnl.sqlite3")};Pooling=False");
            await connection.OpenAsync(token).ConfigureAwait(false);
            using var command = connection.CreateCommand();
            command.CommandText = """
                insert into companies (id, name, enabled) values ('alpha', 'Alpha Engineering', 1), ('beta', 'Beta Labs', 1);
                insert into jobs (company_id, source_id, title, locations_json, countries_json, job_url, apply_url,
                    description, raw_payload, content_hash, eligible, eligibility_reason, source_open, is_new, first_seen_at, last_seen_at)
                values ('alpha','one','Platform Engineer','["Amsterdam"]','["NL"]','https://example.test/jobs/one',
                    'https://example.test/apply/one','Maintain ledgers.','{}','one',1,'eligible',1,0,
                    '2026-08-11T08:00:00+00:00','2026-08-11T10:00:00+00:00'),
                    ('beta','two','Backend Engineer','[]','[]','https://example.test/jobs/two','https://example.test/apply/two',
                    'Build services.','{}','two',1,'eligible',1,0,
                    '2026-08-11T08:00:00+00:00','2026-08-11T10:00:00+00:00');
                """;
            await command.ExecuteNonQueryAsync(token).ConfigureAwait(false);
            return fixture;
        }

        public ValueTask DisposeAsync()
        {
            _directory.Delete(true);
            return ValueTask.CompletedTask;
        }
    }
}
