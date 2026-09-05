using System.Collections.Immutable;
using Avalonia;
using Avalonia.Automation;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Input;
using Avalonia.Input.Raw;
using Avalonia.Media;
using FluentAssertions;
using TechJobsNL.App;
using TechJobsNL.App.Browsing;
using TechJobsNL.Core.Vacancies;
using TechJobsNL.Core.Domain;

[assembly: AvaloniaTestApplication(typeof(TechJobsNL.App.Tests.TestAppBuilder))]

namespace TechJobsNL.App.Tests;

[Trait("TaskId", "V0.1.0-077")]
public sealed class LocalWindowTests
{
    [AvaloniaFact]
    public async Task Close_DuringLoad_CancelsAndObservesWorkBeforeWindowDisappears()
    {
        var source = new PendingSource();
        var model = new LocalWindowViewModel(source);
        var window = new LocalWindow(model);
        window.Show();
        model.State.IsLoading.Should().BeTrue();
        window.FindControl<TextBlock>("StateMessage")!.Text.Should().Contain("Loading");

        window.Close();
        source.Token.IsCancellationRequested.Should().BeTrue();
        window.IsVisible.Should().BeTrue();
        source.Completion.SetException(new IOException("private failure details"));
        await model.CloseAsync();
        await Avalonia.Threading.Dispatcher.UIThread.InvokeAsync(() => { });

        window.IsVisible.Should().BeFalse();
        model.State.Message.Should().NotContain("private");
    }

    [AvaloniaFact]
    public async Task LoadAsync_AfterClose_DoesNotStartWork()
    {
        var source = new PendingSource();
        var model = new LocalWindowViewModel(source);
        await model.CloseAsync();
        await model.LoadAsync();
        source.Calls.Should().Be(0);
    }

    [AvaloniaFact]
    public async Task LoadAsync_FailedRuntime_ShowsSafeUsefulFailure()
    {
        var model = new LocalWindowViewModel(new RuntimeWindowSource(Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"), "missing.toml")));
        var window = new LocalWindow(model);
        window.Show();
        await model.LoadAsync();

        model.State.IsLoading.Should().BeFalse();
        model.State.IsFailed.Should().BeTrue();
        model.State.IsEmpty.Should().BeFalse();
        var message = window.FindControl<TextBlock>("StateMessage");
        Assert.NotNull(message);
        message.IsVisible.Should().BeTrue();
        message.Text.Should().Contain("configuration");
        window.Close();
        await model.CloseAsync();
    }

    [AvaloniaFact]
    public async Task Window_PopulatedSnapshot_SupportsKeyboardSelectionAndVisibleFocus()
    {
        var model = new LocalWindowViewModel(new FixedSource(new LocalWindowLoadResult.Ready([Vacancy("one"), Vacancy("two")])));
        var window = new LocalWindow(model);
        window.Show();
        await model.LoadAsync();
        var list = window.FindControl<ListBox>("VacancyList");
        Assert.NotNull(list);
        list.SelectedIndex = 0;
        window.UpdateLayout();
        var first = list.ContainerFromIndex(0);
        Assert.NotNull(first);
        first.Focus(NavigationMethod.Tab);
        window.KeyPress(Key.Down, RawInputModifiers.None, PhysicalKey.ArrowDown, null);
        window.KeyRelease(Key.Down, RawInputModifiers.None, PhysicalKey.ArrowDown, null);

        list.SelectedIndex.Should().Be(1);
        list.IsKeyboardFocusWithin.Should().BeTrue();
        var selected = Assert.IsType<ListBoxItem>(list.ContainerFromIndex(1));
        AutomationProperties.GetName(selected).Should().Be("Platform Engineer two");
        AutomationProperties.GetHelpText(selected).Should().Be("Alpha Engineering");
        selected.IsFocused.Should().BeTrue();
        selected.BorderThickness.Should().Be(new Thickness(2));
        selected.BorderBrush.Should().BeAssignableTo<ISolidColorBrush>().Which.Color.A.Should().Be(255);
        window.Close();
        await model.CloseAsync();
    }

    [AvaloniaFact]
    public async Task LoadAsync_EmptyLocalData_ShowsUsefulOfflineEmptyState()
    {
        var model = new LocalWindowViewModel(new EmptySource());
        var window = new LocalWindow(model);
        window.Show();
        await model.LoadAsync();

        model.State.IsEmpty.Should().BeTrue();
        model.State.Message.Should().Be("No vacancies are stored on this device yet.");
        window.Title.Should().Be("TechJobsNL");
        await model.CloseAsync();
        window.Close();
    }

    private sealed class EmptySource : ILocalWindowSource
    {
        public Task<LocalWindowLoadResult> LoadAsync(CancellationToken cancellationToken) =>
            Task.FromResult<LocalWindowLoadResult>(new LocalWindowLoadResult.Ready(ImmutableArray<RetainedVacancy>.Empty));
    }

    private sealed class PendingSource : ILocalWindowSource
    {
        public TaskCompletionSource<LocalWindowLoadResult> Completion { get; } = new(TaskCreationOptions.RunContinuationsAsynchronously);
        public CancellationToken Token { get; private set; }
        public int Calls { get; private set; }

        public Task<LocalWindowLoadResult> LoadAsync(CancellationToken cancellationToken)
        {
            Calls++;
            Token = cancellationToken;
            return Completion.Task;
        }
    }

    private static RetainedVacancy Vacancy(string id) => new(
        new VacancyKey(new CompanyId("alpha"), new SourceId(id)), "Alpha Engineering", "Platform Engineer " + id,
        ["Amsterdam"], "Build reliable systems", "https://example.test/job", "https://example.test/apply", true,
        DateTimeOffset.UnixEpoch);

    private sealed class FixedSource : ILocalWindowSource
    {
        private readonly LocalWindowLoadResult _result;

        public FixedSource(LocalWindowLoadResult result) { _result = result; }

        public Task<LocalWindowLoadResult> LoadAsync(CancellationToken cancellationToken) => Task.FromResult(_result);
    }
}
