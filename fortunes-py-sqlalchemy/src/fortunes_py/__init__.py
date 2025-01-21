import contextlib
import multiprocessing

import fastapi
import pydantic
import pydantic_settings
import sqlalchemy.ext.asyncio

from fortunes_py import orm

MAX_CORES = multiprocessing.cpu_count()


class Config(pydantic_settings.BaseSettings):
    # load .env
    model_config = pydantic_settings.SettingsConfigDict(env_file=".env", env_file_encoding="utf-8")

    # asyncpg pool
    database_url: pydantic.PostgresDsn
    database_pool_min_size: int = (MAX_CORES // 2) or 1
    database_pool_max_size: int = (MAX_CORES) or 1


async def get_database(request: fastapi.Request) -> sqlalchemy.ext.asyncio.AsyncEngine:
    """
    Mock this function in tests to avoid committing changes.
    """
    yield request.app.state.database_pool


class Model(pydantic.BaseModel):
    pass


class Fortune(Model):
    content: str


class FortuneListRequest(Model):
    model_config = {
        "json_schema_extra": {
            "examples": [
                {
                    "quantity": 5,
                }
            ]
        }
    }

    quantity: int = pydantic.Field(5, gt=0, lt=21)


class FortuneListResponse(Model):
    count: int
    items: list[Fortune]


async def get_fortunes(session: sqlalchemy.ext.asyncio.AsyncSession, n: int) -> list[Fortune]:
    query = sqlalchemy.select(orm.Fortune).order_by(sqlalchemy.text("random()")).limit(n)
    rows = await session.scalars(query)
    return [Fortune(content=row.content) for row in rows]


async def fortunes(
    request: FortuneListRequest,
    db: sqlalchemy.ext.asyncio.AsyncEngine = fastapi.Depends(get_database),
):
    """
    Return 5 fortunes at random. If quantity=N is passed via the query string,
    return that many items instead.
    """
    async with orm.create_session(db) as session:
        items = await get_fortunes(session, request.quantity)

    return FortuneListResponse(
        count=len(items),
        items=items,
    )


@contextlib.asynccontextmanager
async def lifespan(app: fastapi.FastAPI):
    app.state.database_pool = orm.get_engine(
        app.state.config.database_url,
        app.state.config.database_pool_max_size,
    )

    # force the pool to acquire a connection _now_. If it fails, the database is unavailable.
    async with orm.create_session(app.state.database_pool) as session:
        await session.execute(sqlalchemy.text("SELECT 1=1"))

    yield


def factory():
    app = fastapi.FastAPI(lifespan=lifespan)
    app.state.config = Config()
    app.add_api_route(path="/fortunes", endpoint=fortunes, methods=["POST"])

    return app
