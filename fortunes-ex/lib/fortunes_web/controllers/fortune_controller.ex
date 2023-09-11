defmodule FortunesWeb.FortuneController do
  use FortunesWeb, :controller

  def show(conn, params) do
    fortunes =
      Map.get(params, "quantity", 5)
      |> Fortunes.get()
      |> Enum.map(&%{"content" => &1})

    json(conn, %{count: Enum.count(fortunes), items: fortunes})
  end
end
