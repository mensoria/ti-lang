-- Internal opaque-session vault. Public record identities have no access.
DEFINE TABLE api_session SCHEMAFULL
 PERMISSIONS FOR select WHERE $auth.kind='data_issuer' AND issued_by=$auth.id AND expires_at>time::now()
 FOR create WHERE $auth.kind='data_issuer' AND issued_by=$auth.id AND expires_at>time::now() AND expires_at<=time::now()+301s
 FOR delete WHERE $auth.kind='data_issuer' AND issued_by=$auth.id AND expires_at<=time::now();
DEFINE FIELD issued_by ON api_session TYPE record<data_issuer>;
DEFINE FIELD session ON api_session TYPE object FLEXIBLE;
DEFINE FIELD expires_at ON api_session TYPE datetime;
DEFINE INDEX session_expiry ON api_session FIELDS expires_at;
DEFINE FUNCTION OVERWRITE fn::data_valid() {
 RETURN $auth.kind='data_command' AND $auth.actor_ref.active=true AND $auth.expires_at>time::now() AND $auth.environment=session::db()
 AND (SELECT VALUE state FROM ONLY environment_guard:current)=IF $auth.action='delete' {'CLOSED'} ELSE {'OPEN'}
 AND (SELECT VALUE incarnation FROM ONLY environment_guard:current)=$auth.incarnation
 AND (SELECT VALUE management_generation FROM ONLY environment_guard:current)=$auth.management_generation
 AND (SELECT VALUE applied_security_seq FROM ONLY environment_guard:current)=$auth.security_seq
 AND (SELECT VALUE schema.version FROM ONLY type::record('collection_head',$auth.collection))=$auth.schema_version;
} PERMISSIONS FULL;
