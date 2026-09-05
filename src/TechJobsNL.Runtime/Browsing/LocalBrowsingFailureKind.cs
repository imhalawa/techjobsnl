namespace TechJobsNL.Runtime.Browsing;

/// <summary>The startup stage that prevented local browsing.</summary>
public enum LocalBrowsingFailureKind
{
    Configuration,
    Database,
    Recovery,
    Data,
}
