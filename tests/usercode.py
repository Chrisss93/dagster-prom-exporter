from dagster import asset, define_asset_job, job, op
from dagster import AssetMaterialization, Config, Definitions, ExpectationResult, ScheduleDefinition
import string
from typing import Optional 

class MyConfig(Config):
    name: Optional[str]

@asset
def ascii():
    return list(string.printable)
@asset
def alphabet(context, ascii):
    letters = [i for i in ascii if i.upper() != i]
    context.log_event( ExpectationResult(success=len(letters) < len(ascii)) )
    return letters

ascii_job = define_asset_job(name='ascii_job', selection='ascii')
basic_schedule = ScheduleDefinition(job=ascii_job, cron_schedule='0 0 * * *')

@op
def hello():
    return 'Hello'
@op
def world(context, config: MyConfig):
    s = config.name or 'world'
    context.log_event(
        ExpectationResult(success=len(s) > 0 and s[0].upper() == s[0], label='Proper noun')
    )
    return s
@op
def save_s3(context, greet, name):
    output = f'{{greet}} {{name}}!'
    context.log_event( AssetMaterialization(asset_key='s3.hello_world') )
    return output

@job
def hello_world():
    result = save_s3(hello(), world())
    print(result)

defs = Definitions(
    assets=[ascii, alphabet],
    jobs=[hello_world, ascii_job],
    schedules=[basic_schedule],
)