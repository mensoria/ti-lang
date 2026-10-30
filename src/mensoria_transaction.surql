BEGIN;
IF !fn::ti_service() { THROW 'TI_ACCESS_DENIED'; };
LET $g=SELECT * FROM ONLY environment_guard:current;
LET $u=SELECT * FROM ONLY type::record('user',$actor);
LET $f=SELECT * FROM ONLY type::record('session_family',$family);
LET $h=SELECT * FROM ONLY type::record('ti_head',$module);
IF time::now()>=time::from_secs($expiry) OR $g.state!='OPEN' OR $g.incarnation!=$inc OR $g.applied_security_seq!=$seq OR $u.active!=true OR $u.role!=$role OR ($u.session_epoch??0)!=$epoch OR $f.active!=true OR $f.generation!=$generation OR $f.expires_at<=time::now() { THROW 'TI_ACCESS_DENIED'; };
LET $prior=SELECT * FROM ONLY type::record('operation',$op);
IF $prior!=NONE {
 IF $prior.actor!=$actor {THROW 'TI_ACCESS_DENIED';};
 IF $prior.input_digest!=$digest OR $prior.target_id!=$logical_id {THROW 'TI_IDEMPOTENCY_CONFLICT';};
 IF $prior.expired=true {THROW 'TI_RESULT_EXPIRED';};
 RETURN $prior;
} ELSE {
 IF $h.artifact!=$artifact OR $h.revision!=$head_revision {THROW 'TI_MODEL_STALE';};
 IF !($role IN $h.roles.write) {THROW 'TI_ACCESS_DENIED';};
 LET $old=SELECT * FROM ONLY type::record('ti_record',$rid);
 IF $expected=NULL {IF $old!=NONE {THROW 'TI_REVISION_CONFLICT';};} ELSE {
  IF $old=NONE OR (!($role IN $h.roles.read) AND $old.owner!=$actor) {THROW 'TI_ACCESS_DENIED';};
  IF $old.revision!=$expected {THROW 'TI_REVISION_CONFLICT';};
 };
 LET $guard=UPDATE environment_guard:current SET revision+=1 RETURN AFTER;
 LET $family_written=UPDATE type::record('session_family',$family) SET revision+=1 RETURN AFTER;
 IF array::len($guard)!=1 OR array::len($family_written)!=1 {THROW 'TI_ACCESS_DENIED';};
 LET $written=IF $expected=NULL {CREATE type::record('ti_record',$rid) SET module=$module,entity=$entity,owner=$owner,record=$receipt.candidate.record,revision=$receipt.revision RETURN AFTER} ELSE {UPDATE type::record('ti_record',$rid) SET record=$receipt.candidate.record,revision=$receipt.revision WHERE revision=$expected RETURN AFTER};
 IF array::len($written)!=1 OR $written[0].record!=$receipt.candidate.record {THROW 'TI_REVISION_CONFLICT';};
 IF $fault=1 {THROW 'TI_TEST_ABORT';};
 LET $audit=CREATE type::record('ti_audit',$op) SET actor=$actor,identity_domain='mensoria.end_user/1',scope=$scope,action=$action,logical_id=$logical_id,module=$module,artifact=$artifact,host_fingerprint=$receipt.host_fingerprint,record_id=$rid,old_revision=$expected,new_revision=$receipt.revision,old_state=$old.record.state,new_state=$receipt.candidate.record.state,operation_id=$op RETURN AFTER;
 IF array::len($audit)!=1 {THROW 'TI_ACCESS_DENIED';};
 IF $fault=2 {THROW 'TI_TEST_ABORT';};
 LET $operation=CREATE type::record('operation',$op) SET receipt_class='ti_action',operation_id=$op,actor=$actor,owner=$owner,entity=$entity,module=$module,target_id=$logical_id,input_digest=$digest,incarnation=$inc,artifact=$artifact,host_fingerprint=$receipt.host_fingerprint,receipt=$receipt,state='SUCCEEDED',expired=false RETURN AFTER;
 LET $outbox=CREATE type::record('outbox',$op) SET receipt_class='ti_action',operation_id=$op,actor=$actor,input_digest=$digest,incarnation=$inc,module=$module,envelopes=$envelopes,event_type='TI_ACTION',artifact=$artifact,revision=$receipt.revision,events=$receipt.candidate.events,attempt_state='PENDING' RETURN AFTER;
 IF array::len($operation)!=1 OR array::len($outbox)!=1 {THROW 'TI_ACCESS_DENIED';};
 IF $fault=3 {THROW 'TI_TEST_ABORT';};
 RETURN $operation[0];
};
COMMIT;
