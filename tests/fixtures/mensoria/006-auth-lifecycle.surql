-- Upgrade in a maintenance window. Old sessions have no family and must reauthenticate.
DELETE api_session;
DEFINE ACCESS api_account ON DATABASE TYPE RECORD
 SIGNIN (SELECT * FROM user WHERE email=$email AND active=true AND crypto::argon2::compare(pass,$pass))
 DURATION FOR TOKEN 24h, FOR SESSION 24h;
DEFINE ACCESS api_admin ON DATABASE TYPE RECORD
 SIGNIN (SELECT * FROM schema_admin WHERE email=$email AND active=true AND crypto::argon2::compare(pass,$pass))
 DURATION FOR TOKEN 24h, FOR SESSION 24h;
DEFINE TABLE session_family SCHEMAFULL
 PERMISSIONS FOR select WHERE ($auth.kind='data_issuer' AND issued_by=$auth.id) OR ($auth.kind='data_command' AND id=$auth.family_ref)
 FOR create WHERE $auth.kind='data_issuer' AND issued_by=$auth.id
 FOR update WHERE ($auth.kind='data_issuer' AND issued_by=$auth.id) OR ($auth.kind='data_command' AND id=$auth.family_ref AND active=true AND generation=$auth.session_generation);
DEFINE FIELD issued_by ON session_family TYPE record<data_issuer> PERMISSIONS FOR update NONE;
DEFINE FIELD active ON session_family TYPE bool PERMISSIONS FOR update WHERE $auth.kind='data_issuer';
DEFINE FIELD generation ON session_family TYPE int PERMISSIONS FOR update WHERE $auth.kind='data_issuer';
DEFINE FIELD revision ON session_family TYPE int;
DEFINE FIELD refresh_hash ON session_family TYPE string PERMISSIONS FOR select, update WHERE $auth.kind='data_issuer';
DEFINE FIELD expires_at ON session_family TYPE datetime PERMISSIONS FOR update WHERE $auth.kind='data_issuer';
DEFINE FIELD session ON session_family TYPE object FLEXIBLE PERMISSIONS FOR select, update WHERE $auth.kind='data_issuer';
DEFINE TABLE refresh_used SCHEMALESS PERMISSIONS FOR select, create WHERE $auth.kind='data_issuer' AND issued_by=$auth.id;
DEFINE TABLE login_bucket SCHEMALESS PERMISSIONS FOR select, create, update WHERE $auth.kind='data_issuer' AND issued_by=$auth.id;
DEFINE FIELD family_ref ON api_session TYPE option<record<session_family>>;
DEFINE FIELD family_ref ON data_command TYPE option<record<session_family>>;
DEFINE FIELD session_generation ON data_command TYPE option<int>;
DEFINE FUNCTION OVERWRITE fn::data_valid() {
 RETURN $auth.kind='data_command' AND $auth.actor_ref.active=true AND $auth.expires_at>time::now() AND $auth.environment=session::db()
 AND $auth.family_ref!=NONE AND $auth.family_ref.active=true AND $auth.family_ref.generation=$auth.session_generation AND $auth.family_ref.expires_at>time::now()
 AND (SELECT VALUE state FROM ONLY environment_guard:current)=IF $auth.action='delete' {'CLOSED'} ELSE {'OPEN'}
 AND (SELECT VALUE incarnation FROM ONLY environment_guard:current)=$auth.incarnation
 AND (SELECT VALUE management_generation FROM ONLY environment_guard:current)=$auth.management_generation
 AND (SELECT VALUE applied_security_seq FROM ONLY environment_guard:current)=$auth.security_seq
 AND (SELECT VALUE schema.version FROM ONLY type::record('collection_head',$auth.collection))=$auth.schema_version;
} PERMISSIONS FULL;
