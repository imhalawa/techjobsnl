using FluentAssertions;
using TechJobsNL.Core.Application.Dispatch;

namespace TechJobsNL.Core.Tests.Application.Dispatch;

public sealed class DispatchRegistryTests
{
    [Fact]
    [Trait("TaskId", "V0.1.0-004")]
    [Trait("Category", "Unit")]
    public async Task ExecuteAsync_RegisteredCommand_AppliesLoggingTimingValidationAndHandlerInOrder()
    {
        var trace = new List<string>();
        var timeProvider = new RecordingTimeProvider(trace);
        var handler = new RecordingCommandHandler(trace);
        var dispatchers = new DispatchRegistryBuilder(new RecordingLogger(trace), timeProvider)
            .RegisterCommand<ExampleCommand, string>(handler, new RecordingCommandValidator(trace, timeProvider))
            .Build();

        var result = await dispatchers.Commands.ExecuteAsync(
            new ExampleCommand("saved"),
            TestContext.Current.CancellationToken);

        result.Should().BeOfType<DispatchResult<string>.Success>().Which.Value.Should().Be("saved");
        trace.Should().StartWith("logging", "timing", "validation", "handler");
        trace.Should().EndWith("completed");
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-004")]
    [Trait("Category", "Unit")]
    public async Task ExecuteAsync_InvalidCommand_ReturnsTypedValidationFailureWithoutCallingHandler()
    {
        var handler = new RecordingCommandHandler([]);
        var dispatchers = new DispatchRegistryBuilder(new RecordingLogger([]), TimeProvider.System)
            .RegisterCommand<ExampleCommand, string>(handler, new InvalidCommandValidator())
            .Build();

        var result = await dispatchers.Commands.ExecuteAsync(
            new ExampleCommand("invalid"),
            TestContext.Current.CancellationToken);

        result.Should().BeOfType<DispatchResult<string>.Failure>()
            .Which.Reason.Should().BeOfType<DispatchFailure.ValidationFailed>()
            .Which.Code.Should().Be("invalid-command");
        handler.Calls.Should().Be(0);
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-004")]
    [Trait("Category", "Unit")]
    public void Build_DuplicateCommandAndQueryRegistrations_ThrowTypedFailures()
    {
        var commandBuilder = new DispatchRegistryBuilder(new RecordingLogger([]), TimeProvider.System)
            .RegisterCommand<ExampleCommand, string>(new RecordingCommandHandler([]), new ValidCommandValidator());
        var queryBuilder = new DispatchRegistryBuilder(new RecordingLogger([]), TimeProvider.System)
            .RegisterQuery<ExampleQuery, string>(new RecordingQueryHandler(), new ValidQueryValidator());

        var commandAction = () => commandBuilder.RegisterCommand<ExampleCommand, string>(
            new RecordingCommandHandler([]),
            new ValidCommandValidator());
        var queryAction = () => queryBuilder.RegisterQuery<ExampleQuery, string>(
            new RecordingQueryHandler(),
            new ValidQueryValidator());

        commandAction.Should().Throw<DuplicateRegistrationException>()
            .Which.RequestType.Should().Contain(nameof(ExampleCommand));
        queryAction.Should().Throw<DuplicateRegistrationException>()
            .Which.RequestType.Should().Contain(nameof(ExampleQuery));
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-004")]
    [Trait("Category", "Unit")]
    public async Task Dispatch_UnregisteredCommandAndQuery_ReturnsTypedMissingHandlerFailures()
    {
        var dispatchers = new DispatchRegistryBuilder(new RecordingLogger([]), TimeProvider.System).Build();

        var commandResult = await dispatchers.Commands.ExecuteAsync(
            new ExampleCommand("missing"),
            TestContext.Current.CancellationToken);
        var queryResult = await dispatchers.Queries.QueryAsync(
            new ExampleQuery("missing"),
            TestContext.Current.CancellationToken);

        commandResult.Should().BeOfType<DispatchResult<string>.Failure>()
            .Which.Reason.Should().BeOfType<DispatchFailure.MissingHandler>()
            .Which.RequestType.Should().Contain(nameof(ExampleCommand));
        queryResult.Should().BeOfType<DispatchResult<string>.Failure>()
            .Which.Reason.Should().BeOfType<DispatchFailure.MissingHandler>()
            .Which.RequestType.Should().Contain(nameof(ExampleQuery));
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-004")]
    [Trait("Category", "Unit")]
    public async Task ExecuteAsync_PreCancelledToken_PropagatesCancellationWithoutStartingValidation()
    {
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();
        var validator = new ValidCommandValidator();
        var handler = new RecordingCommandHandler([]);
        var dispatchers = new DispatchRegistryBuilder(new RecordingLogger([]), TimeProvider.System)
            .RegisterCommand<ExampleCommand, string>(handler, validator)
            .Build();

        var action = async () => await dispatchers.Commands.ExecuteAsync(
            new ExampleCommand("cancelled"),
            cancellation.Token).ConfigureAwait(false);

        await action.Should().ThrowAsync<OperationCanceledException>().ConfigureAwait(true);
        validator.Calls.Should().Be(0);
        handler.Calls.Should().Be(0);
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-004")]
    [Trait("Category", "Unit")]
    public async Task ExecuteAsync_HandlerCancellation_PropagatesCancellationWithoutAnExpectedFailure()
    {
        using var cancellation = new CancellationTokenSource();
        var handler = new CancellingCommandHandler();
        var dispatchers = new DispatchRegistryBuilder(new RecordingLogger([]), TimeProvider.System)
            .RegisterCommand<ExampleCommand, string>(handler, new ValidCommandValidator())
            .Build();

        var execution = dispatchers.Commands.ExecuteAsync(new ExampleCommand("waiting"), cancellation.Token);
        await handler.Started.Task.WaitAsync(TestContext.Current.CancellationToken).ConfigureAwait(true);
        cancellation.Cancel();

        var action = async () => await execution.ConfigureAwait(false);

        await action.Should().ThrowAsync<OperationCanceledException>().ConfigureAwait(true);
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-004")]
    [Trait("Category", "Unit")]
    public void CoreDispatch_ContainsNoMediatorReflectionOrAssemblyScanningRegistration()
    {
        var repositoryRoot = FindRepositoryRoot();
        var source = Directory
            .GetFiles(Path.Combine(repositoryRoot.FullName, "src", "TechJobsNL.Core"), "*.cs", SearchOption.AllDirectories)
            .Select(File.ReadAllText)
            .ToArray();

        source.Should().NotContain(text => text.Contains("MediatR", StringComparison.Ordinal));
        source.Should().NotContain(text => text.Contains("Assembly.Get", StringComparison.Ordinal));
        source.Should().NotContain(text => text.Contains(".GetTypes(", StringComparison.Ordinal));
        source.Should().NotContain(text => text.Contains("ScanAssembly", StringComparison.Ordinal));
    }

    private static DirectoryInfo FindRepositoryRoot()
    {
        DirectoryInfo? directory = new(AppContext.BaseDirectory);

        while (directory is not null)
        {
            if (File.Exists(Path.Combine(directory.FullName, "TechJobsNL.slnx")))
            {
                return directory;
            }

            directory = directory.Parent;
        }

        throw new DirectoryNotFoundException("Could not find the repository root.");
    }

    private sealed class ExampleCommand : ICommand<string>
    {
        public ExampleCommand(string value)
        {
            Value = value;
        }

        public string Value { get; }
    }

    private sealed class ExampleQuery : IQuery<string>
    {
        public ExampleQuery(string value)
        {
            Value = value;
        }

        public string Value { get; }
    }

    private sealed class RecordingCommandHandler : ICommandHandler<ExampleCommand, string>
    {
        private readonly List<string> _trace;

        public RecordingCommandHandler(List<string> trace)
        {
            _trace = trace;
        }

        public int Calls { get; private set; }

        public Task<DispatchResult<string>> ExecuteAsync(ExampleCommand command, CancellationToken cancellationToken)
        {
            Calls++;
            _trace.Add("handler");
            return Task.FromResult<DispatchResult<string>>(new DispatchResult<string>.Success(command.Value));
        }
    }

    private sealed class RecordingCommandValidator : IRequestValidator<ExampleCommand>
    {
        private readonly TimeProvider _timeProvider;
        private readonly List<string> _trace;

        public RecordingCommandValidator(List<string> trace, TimeProvider timeProvider)
        {
            _trace = trace;
            _timeProvider = timeProvider;
        }

        public Task<ValidationResult> ValidateAsync(ExampleCommand request, CancellationToken cancellationToken)
        {
            _timeProvider.GetTimestamp().Should().BeGreaterThan(0);
            _trace.Should().Contain("logging");
            _trace.Add("validation");
            return Task.FromResult(ValidationResult.ValidResult);
        }
    }

    private sealed class ValidCommandValidator : IRequestValidator<ExampleCommand>
    {
        public int Calls { get; private set; }

        public Task<ValidationResult> ValidateAsync(ExampleCommand request, CancellationToken cancellationToken)
        {
            Calls++;
            return Task.FromResult(ValidationResult.ValidResult);
        }
    }

    private sealed class InvalidCommandValidator : IRequestValidator<ExampleCommand>
    {
        public Task<ValidationResult> ValidateAsync(ExampleCommand request, CancellationToken cancellationToken) =>
            Task.FromResult<ValidationResult>(new ValidationResult.Invalid("invalid-command", "The command is invalid."));
    }

    private sealed class RecordingQueryHandler : IQueryHandler<ExampleQuery, string>
    {
        public Task<DispatchResult<string>> QueryAsync(ExampleQuery query, CancellationToken cancellationToken) =>
            Task.FromResult<DispatchResult<string>>(new DispatchResult<string>.Success(query.Value));
    }

    private sealed class ValidQueryValidator : IRequestValidator<ExampleQuery>
    {
        public Task<ValidationResult> ValidateAsync(ExampleQuery request, CancellationToken cancellationToken) =>
            Task.FromResult(ValidationResult.ValidResult);
    }

    private sealed class CancellingCommandHandler : ICommandHandler<ExampleCommand, string>
    {
        public CancellingCommandHandler()
        {
            Started = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        }

        public TaskCompletionSource Started { get; }

        public async Task<DispatchResult<string>> ExecuteAsync(ExampleCommand command, CancellationToken cancellationToken)
        {
            Started.SetResult();
            await Task.Delay(Timeout.InfiniteTimeSpan, cancellationToken).ConfigureAwait(false);
            return new DispatchResult<string>.Success(command.Value);
        }
    }

    private sealed class RecordingLogger : IDispatchLogger
    {
        private readonly List<string> _trace;

        public RecordingLogger(List<string> trace)
        {
            _trace = trace;
        }

        public void Started(string requestType) => _trace.Add("logging");

        public void Completed(string requestType, TimeSpan elapsed, bool succeeded) => _trace.Add("completed");
    }

    private sealed class RecordingTimeProvider : TimeProvider
    {
        private readonly List<string> _trace;
        private long _timestamp;

        public RecordingTimeProvider(List<string> trace)
        {
            _trace = trace;
        }

        public override long GetTimestamp()
        {
            _trace.Add("timing");
            _timestamp++;
            return _timestamp;
        }
    }
}
