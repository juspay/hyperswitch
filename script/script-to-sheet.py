import pandas as pd

csv_file = "extracted_request_ids.csv"
xlsx_file = "extracted_request_ids.xlsx"

df = pd.read_csv(csv_file)
df.to_excel(xlsx_file, index=False)
