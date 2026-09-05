namespace TechJobsNL.Core.Application.Dispatch;

/// <summary>Validates one request type before its handler is called.</summary>
/// <typeparam name="TRequest">The command or query being validated.</typeparam>
public interface IRequestValidator<in TRequest>
{
    /// <summary>Returns an expected validation outcome without throwing for invalid input.</summary>
    Task<ValidationResult> ValidateAsync(TRequest request, CancellationToken cancellationToken);
}
