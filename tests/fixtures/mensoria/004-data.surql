DEFINE TABLE schema_admin SCHEMAFULL PERMISSIONS FOR select WHERE id=$auth.id;
DEFINE FIELD email ON schema_admin TYPE string;
DEFINE FIELD pass ON schema_admin TYPE string PERMISSIONS NONE;
DEFINE FIELD active ON schema_admin TYPE bool DEFAULT true;
DEFINE FIELD kind ON schema_admin TYPE string DEFAULT 'schema_admin';
DEFINE ACCESS schema_admin_access ON DATABASE TYPE RECORD
 SIGNIN (SELECT * FROM schema_admin WHERE email=$email AND active=true AND crypto::argon2::compare(pass,$pass))
 DURATION FOR TOKEN 5m, FOR SESSION 5m;
DEFINE TABLE collection_head SCHEMALESS PERMISSIONS FOR select WHERE $auth.active=true OR $auth.kind IN ['data_issuer','data_command'];
DEFINE TABLE data_issuer SCHEMAFULL PERMISSIONS FOR select WHERE id=$auth.id;
DEFINE FIELD kind ON data_issuer TYPE string DEFAULT 'data_issuer';
DEFINE FIELD environment ON data_issuer TYPE string;
DEFINE FIELD pass ON data_issuer TYPE string PERMISSIONS NONE;
DEFINE ACCESS data_issuer_access ON DATABASE TYPE RECORD
 SIGNIN (SELECT * FROM data_issuer WHERE environment=$environment AND crypto::argon2::compare(pass,$password))
 DURATION FOR TOKEN 5m, FOR SESSION 5m;
DEFINE TABLE data_command SCHEMAFULL
 PERMISSIONS FOR select WHERE id=$auth.id OR ($auth.kind='data_issuer' AND issued_by=$auth.id)
 FOR create WHERE $auth.kind='data_issuer' AND issued_by=$auth.id AND environment=$auth.environment AND actor_ref.active=true
  AND expires_at>time::now() AND expires_at<=time::now()+30s;
DEFINE FIELD issued_by ON data_command TYPE record<data_issuer>;
DEFINE FIELD actor_ref ON data_command TYPE record<user>;
DEFINE FIELD kind ON data_command TYPE string DEFAULT 'data_command';
DEFINE FIELD environment ON data_command TYPE string;
DEFINE FIELD incarnation ON data_command TYPE int;
DEFINE FIELD security_seq ON data_command TYPE int;
DEFINE FIELD management_generation ON data_command TYPE int;
DEFINE FIELD collection ON data_command TYPE string;
DEFINE FIELD schema_version ON data_command TYPE int;
DEFINE FIELD resource_id ON data_command TYPE record;
DEFINE FIELD action ON data_command TYPE string ASSERT $value IN ['create','patch','delete'];
DEFINE FIELD expected_revision ON data_command TYPE option<int>;
DEFINE FIELD payload ON data_command TYPE object FLEXIBLE;
DEFINE FIELD operation_id ON data_command TYPE string;
DEFINE FIELD input_digest ON data_command TYPE string;
DEFINE FIELD expires_at ON data_command TYPE datetime;
DEFINE FIELD pass ON data_command TYPE string PERMISSIONS FOR select NONE FOR create FULL FOR update NONE;
DEFINE ACCESS data_command_access ON DATABASE TYPE RECORD
 SIGNIN (SELECT * FROM data_command WHERE id=type::record('data_command',$id) AND expires_at>time::now() AND actor_ref.active=true AND crypto::argon2::compare(pass,$password))
 DURATION FOR TOKEN 30s, FOR SESSION 30s;
DEFINE TABLE OVERWRITE user SCHEMAFULL PERMISSIONS FOR select WHERE id=$auth.id OR ($auth.kind IN ['command','data_command'] AND id=$auth.actor_ref);
DEFINE FUNCTION fn::data_valid() {
 RETURN $auth.kind='data_command' AND $auth.actor_ref.active=true AND $auth.expires_at>time::now() AND $auth.environment=session::db()
 AND (SELECT VALUE state FROM ONLY environment_guard:current)='CLOSED'
 AND (SELECT VALUE incarnation FROM ONLY environment_guard:current)=$auth.incarnation
 AND (SELECT VALUE management_generation FROM ONLY environment_guard:current)=$auth.management_generation
 AND (SELECT VALUE applied_security_seq FROM ONLY environment_guard:current)=$auth.security_seq
 AND (SELECT VALUE schema.version FROM ONLY type::record('collection_head',$auth.collection))=$auth.schema_version;
} PERMISSIONS FULL;
DEFINE TABLE OVERWRITE environment_guard SCHEMAFULL PERMISSIONS FOR select WHERE $auth.id!=NONE
 FOR update WHERE ($auth.active=true AND state='OPEN') OR fn::command_valid() OR fn::data_valid();
DEFINE TABLE OVERWRITE operation SCHEMALESS
 PERMISSIONS FOR select WHERE (fn::command_valid() AND id=type::record('operation',$auth.operation_id) AND actor_ref=$auth.actor_ref)
  OR (fn::data_valid() AND id=type::record('operation',$auth.operation_id) AND actor_ref=$auth.actor_ref)
  OR ($auth.active=true AND actor_ref=$auth.id)
 FOR create WHERE (fn::command_valid() AND id=type::record('operation',$auth.operation_id) AND actor_ref=$auth.actor_ref AND resource_id=$auth.resource_id AND input_digest=$auth.input_digest AND incarnation=$auth.incarnation AND state='SUCCEEDED' AND resource_revision=$auth.resource_id.revision AND crypto::sha256($auth.resource_id.title)=$auth.title_digest)
  OR (fn::data_valid() AND id=type::record('operation',$auth.operation_id) AND actor_ref=$auth.actor_ref AND resource_id=$auth.resource_id AND input_digest=$auth.input_digest AND incarnation=$auth.incarnation AND state='SUCCEEDED' AND revision=$auth.resource_id.revision);
DEFINE TABLE OVERWRITE outbox SCHEMALESS
 PERMISSIONS FOR select WHERE ((fn::command_valid() OR fn::data_valid()) AND id=type::record('outbox',$auth.operation_id) AND actor_ref=$auth.actor_ref)
 FOR create WHERE ((fn::command_valid() OR fn::data_valid()) AND id=type::record('outbox',$auth.operation_id) AND operation_id=$auth.operation_id AND actor_ref=$auth.actor_ref AND input_digest=$auth.input_digest AND resource_id=$auth.resource_id AND (SELECT VALUE input_digest FROM ONLY type::record('operation',$auth.operation_id))=$auth.input_digest);

DEFINE TABLE data_tombstone SCHEMALESS PERMISSIONS FOR select WHERE IF $auth.kind='data_command' { fn::data_valid() AND id=type::record('data_tombstone',crypto::sha256($auth.collection+':'+<string>record::id($auth.resource_id))) } ELSE { false };
