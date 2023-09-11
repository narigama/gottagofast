defmodule Fortunes do
  import Ecto.Query, warn: false
  alias Fortunes.{Repo, Fortune}

  def get(limit \\ 5) do
    Repo.all(from f in Fortune, order_by: fragment("RANDOM()"), limit: ^limit, select: f.content)
  end
end
