using TechJobsNL.Core.Domain;

namespace TechJobsNL.Adapters.Providers.Http;

/// <summary>Describes an adapter failure using stable domain categories and retry metadata.</summary>
public sealed record ProviderFailure(SourceErrorKind Kind, string Message, int? HttpStatus, TimeSpan? RetryAfter, bool IsRetryable);
