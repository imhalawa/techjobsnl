using System.Text.Json;
using System.Text.Json.Serialization;

namespace TechJobsNL.Adapters.Providers.Normalization;

public sealed record JobPosting(
    [property: JsonPropertyName("identifier")] JsonElement Identifier,
    [property: JsonPropertyName("url")] string? Url,
    [property: JsonPropertyName("title")] string? Title,
    [property: JsonPropertyName("description")] string Description,
    [property: JsonPropertyName("employmentType")] JsonElement EmploymentType,
    [property: JsonPropertyName("datePosted")] string? DatePosted,
    [property: JsonPropertyName("jobLocation")] JsonElement JobLocation,
    [property: JsonPropertyName("hiringOrganization")] JsonElement HiringOrganization);
