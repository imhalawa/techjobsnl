namespace TechJobsNL.Core.Domain;

/// <summary>Describes a source failure without exposing exception or transport implementation details.</summary>
public sealed record ScanFailure(SourceErrorKind Kind, string Diagnostic);
