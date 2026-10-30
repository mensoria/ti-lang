-- Fixed document-command capability bridge. Privileged installation only.
DEFINE TABLE command_issuer SCHEMAFULL PERMISSIONS FOR select WHERE id=$auth.id;
DEFINE FIELD kind ON command_issuer TYPE string ASSERT $value = 'issuer';
DEFINE FIELD environment ON command_issuer TYPE string;
DEFINE FIELD pass ON command_issuer TYPE string PERMISSIONS NONE;
DEFINE ACCESS command_issuer_access ON DATABASE TYPE RECORD
 SIGNIN (SELECT * FROM command_issuer WHERE environment=$environment AND crypto::argon2::compare(pass,$password))
 DURATION FOR TOKEN 5m, FOR SESSION 5m;

DEFINE TABLE command_session SCHEMAFULL
 PERMISSIONS FOR select WHERE id=$auth.id OR ($auth.kind='issuer' AND issued_by=$auth.id)
 FOR create WHERE $auth.kind='issuer' AND issued_by=$auth.id AND environment=$auth.environment
   AND kind='command' AND command_class='document_put' AND actor_ref.active=true
   AND expires_at>time::now() AND expires_at<=time::now()+30s;
DEFINE FIELD kind ON command_session TYPE string ASSERT $value='command';
DEFINE FIELD issued_by ON command_session TYPE record<command_issuer>;
DEFINE FIELD actor_ref ON command_session TYPE record<user>;
DEFINE FIELD environment ON command_session TYPE string;
DEFINE FIELD incarnation ON command_session TYPE int;
DEFINE FIELD security_seq ON command_session TYPE int;
DEFINE FIELD command_class ON command_session TYPE string ASSERT $value='document_put';
DEFINE FIELD operation_id ON command_session TYPE string;
DEFINE FIELD resource_id ON command_session TYPE record<doc>;
DEFINE FIELD expected_revision ON command_session TYPE option<int>;
DEFINE FIELD input_digest ON command_session TYPE string;
DEFINE FIELD title_digest ON command_session TYPE string;
DEFINE FIELD expires_at ON command_session TYPE datetime;
DEFINE FIELD pass ON command_session TYPE string PERMISSIONS FOR select NONE FOR create FULL FOR update NONE;
DEFINE ACCESS native_command ON DATABASE TYPE RECORD
 SIGNIN (SELECT * FROM command_session WHERE id=type::record('command_session',$id)
   AND expires_at>time::now() AND actor_ref.active=true AND crypto::argon2::compare(pass,$password))
 DURATION FOR TOKEN 30s, FOR SESSION 30s;

DEFINE TABLE OVERWRITE user SCHEMAFULL
 PERMISSIONS FOR select WHERE id=$auth.id OR ($auth.kind='command' AND id=$auth.actor_ref);

DEFINE FUNCTION fn::command_valid() {
 RETURN $auth.kind='command' AND $auth.command_class='document_put'
   AND $auth.expires_at>time::now() AND $auth.actor_ref.active=true
   AND $auth.environment=session::db()
   AND (SELECT VALUE state FROM ONLY environment_guard:current)='OPEN'
   AND (SELECT VALUE incarnation FROM ONLY environment_guard:current)=$auth.incarnation
   AND (SELECT VALUE applied_security_seq FROM ONLY environment_guard:current)=$auth.security_seq;
} PERMISSIONS FULL;

DEFINE TABLE OVERWRITE doc SCHEMAFULL
 PERMISSIONS FOR select WHERE (owner=$auth.id AND $auth.active=true)
   OR (fn::command_valid() AND owner=$auth.actor_ref AND id=$auth.resource_id)
 FOR create WHERE (owner=$auth.id AND $auth.active=true)
   OR (fn::command_valid() AND owner=$auth.actor_ref AND id=$auth.resource_id
       AND $auth.expected_revision=NONE AND crypto::sha256(title)=$auth.title_digest AND revision=1)
 FOR update WHERE (owner=$auth.id AND $auth.active=true)
   OR (fn::command_valid() AND owner=$auth.actor_ref AND id=$auth.resource_id AND $auth.expected_revision!=NONE)
 FOR delete WHERE owner=$auth.id AND $auth.active=true;
DEFINE FIELD OVERWRITE title ON doc TYPE string
 PERMISSIONS FOR select FULL FOR create, update WHERE $auth.kind!='command' OR crypto::sha256($after)=$auth.title_digest;
DEFINE FIELD OVERWRITE owner ON doc TYPE record<user>
 PERMISSIONS FOR select FULL FOR create FULL FOR update WHERE $auth.kind!='command';
DEFINE FIELD OVERWRITE secret ON doc TYPE option<string>
 PERMISSIONS FOR select WHERE $auth.role='admin' OR (fn::command_valid() AND $auth.actor_ref.role='admin')
 FOR create, update WHERE $auth.kind!='command';
DEFINE FIELD revision ON doc TYPE int DEFAULT 0
 PERMISSIONS FOR select FULL FOR create FULL
 FOR update WHERE $auth.kind!='command' OR ($before=$auth.expected_revision AND $after=$before+1);
UPDATE doc SET revision=0 WHERE revision=NONE;

DEFINE TABLE OVERWRITE environment_guard SCHEMAFULL
 PERMISSIONS FOR select WHERE $auth.id!=NONE
 FOR update WHERE ($auth.active=true AND state='OPEN') OR fn::command_valid();
DEFINE FIELD OVERWRITE revision ON environment_guard TYPE int
 PERMISSIONS FOR select FULL FOR create NONE FOR update WHERE $after=$before+1;

-- Existing root-only qualification fixtures remain readable to root. New record writes are narrow.
DEFINE TABLE OVERWRITE operation SCHEMALESS
 PERMISSIONS FOR select WHERE fn::command_valid() AND id=type::record('operation',$auth.operation_id)
   AND actor_ref=$auth.actor_ref
 FOR create WHERE fn::command_valid() AND id=type::record('operation',$auth.operation_id)
   AND actor_ref=$auth.actor_ref AND resource_id=$auth.resource_id AND input_digest=$auth.input_digest
   AND incarnation=$auth.incarnation AND state='SUCCEEDED'
   AND resource_revision=$auth.resource_id.revision AND crypto::sha256($auth.resource_id.title)=$auth.title_digest;
DEFINE TABLE OVERWRITE outbox SCHEMALESS
 PERMISSIONS FOR select WHERE fn::command_valid() AND id=type::record('outbox',$auth.operation_id)
   AND operation_id=$auth.operation_id AND actor_ref=$auth.actor_ref
 FOR create WHERE fn::command_valid() AND id=type::record('outbox',$auth.operation_id)
   AND operation_id=$auth.operation_id AND actor_ref=$auth.actor_ref AND input_digest=$auth.input_digest
   AND resource_id=$auth.resource_id AND event_type='DOCUMENT_WRITTEN'
   AND (SELECT VALUE input_digest FROM ONLY type::record('operation',$auth.operation_id))=$auth.input_digest;
