using TechJobsNL.Core.Domain;

namespace TechJobsNL.Core.Profiles;

public sealed record CompanyProfileFacts(CompanyId Id, string Name, bool IsFollowed, Uri OfficialSource, CompanySourceHealth Health, string? Diagnostic);
